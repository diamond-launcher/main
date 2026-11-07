//! Verified, concurrent file downloads with real byte-level progress.
use crate::error::{AppError, Result};
use futures_util::{stream, StreamExt};
use sha1::{Digest, Sha1};
use std::collections::HashSet;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncWriteExt;

const CONCURRENCY: usize = 16;
const ATTEMPTS: u32 = 3;

#[derive(Debug, Clone)]
pub struct Job {
    pub url: String,
    pub dest: PathBuf,
    pub sha1: Option<String>,
    pub size: Option<u64>,
}

#[derive(Default)]
pub struct Progress {
    pub bytes: AtomicU64,
    pub files: AtomicU64,
}

pub fn sha1_file(path: &Path) -> std::io::Result<String> {
    let mut file = std::io::BufReader::new(std::fs::File::open(path)?);
    let mut hasher = Sha1::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

/// Cheap check: file exists and has the expected size.
pub fn is_present(job: &Job) -> bool {
    std::fs::metadata(&job.dest).is_ok_and(|m| m.is_file() && job.size.map_or(true, |s| m.len() == s))
}

/// Full check: presence plus SHA-1 when the metadata provides one.
pub fn is_intact(job: &Job) -> bool {
    if !is_present(job) {
        return false;
    }
    match &job.sha1 {
        Some(expected) => sha1_file(&job.dest).is_ok_and(|actual| actual.eq_ignore_ascii_case(expected)),
        None => true,
    }
}

/// Returns the (deduplicated) jobs whose files are missing or corrupt.
pub async fn pending(jobs: Vec<Job>, deep: bool) -> Result<Vec<Job>> {
    let mut seen = HashSet::new();
    let unique: Vec<Job> = jobs.into_iter().filter(|j| seen.insert(j.dest.clone())).collect();
    let checked: Vec<(Job, bool)> = stream::iter(unique)
        .map(|job| async move {
            let probe = job.clone();
            let ok = tokio::task::spawn_blocking(move || if deep { is_intact(&probe) } else { is_present(&probe) })
                .await
                .unwrap_or(false);
            (job, ok)
        })
        .buffer_unordered(8)
        .collect()
        .await;
    Ok(checked.into_iter().filter(|(_, ok)| !ok).map(|(j, _)| j).collect())
}

pub async fn download_all(client: &reqwest::Client, jobs: Vec<Job>, progress: Arc<Progress>) -> Result<()> {
    let mut results = stream::iter(jobs)
        .map(|job| {
            let client = client.clone();
            let progress = progress.clone();
            async move { fetch(&client, &job, &progress).await }
        })
        .buffer_unordered(CONCURRENCY);
    while let Some(result) = results.next().await {
        result?;
    }
    Ok(())
}

async fn fetch(client: &reqwest::Client, job: &Job, progress: &Progress) -> Result<()> {
    if let Some(parent) = job.dest.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let mut last_error = AppError::new("network", "Download failed");
    for attempt in 1..=ATTEMPTS {
        let mut counted = 0u64;
        match attempt_once(client, job, progress, &mut counted).await {
            Ok(()) => {
                progress.files.fetch_add(1, Ordering::Relaxed);
                return Ok(());
            }
            Err(e) => {
                progress.bytes.fetch_sub(counted, Ordering::Relaxed);
                last_error = e;
                if attempt < ATTEMPTS {
                    tokio::time::sleep(Duration::from_millis(400 * u64::from(attempt))).await;
                }
            }
        }
    }
    Err(AppError::new(last_error.code, format!("{} ({})", last_error.message, job.url)))
}

async fn attempt_once(client: &reqwest::Client, job: &Job, progress: &Progress, counted: &mut u64) -> Result<()> {
    let response = client.get(&job.url).send().await?.error_for_status()?;
    let mut tmp_name = job.dest.file_name().unwrap_or_default().to_os_string();
    tmp_name.push(".part");
    let tmp = job.dest.with_file_name(tmp_name);

    let mut file = tokio::fs::File::create(&tmp).await?;
    let mut hasher = Sha1::new();
    let mut total = 0u64;
    let mut body = response.bytes_stream();
    while let Some(chunk) = body.next().await {
        let chunk = chunk?;
        hasher.update(&chunk);
        file.write_all(&chunk).await?;
        let len = chunk.len() as u64;
        total += len;
        *counted += len;
        progress.bytes.fetch_add(len, Ordering::Relaxed);
    }
    file.flush().await?;
    drop(file);

    if let Some(size) = job.size {
        if size != total {
            let _ = tokio::fs::remove_file(&tmp).await;
            return Err(AppError::new("checksum", format!("Size mismatch for {}", job.dest.display())));
        }
    }
    if let Some(expected) = &job.sha1 {
        if !hex::encode(hasher.finalize()).eq_ignore_ascii_case(expected) {
            let _ = tokio::fs::remove_file(&tmp).await;
            return Err(AppError::new("checksum", format!("Checksum mismatch for {}", job.dest.display())));
        }
    }
    tokio::fs::rename(&tmp, &job.dest).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn pending_detects_missing_and_corrupt_files() {
        let dir = std::env::temp_dir().join(format!("diamond-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let good = dir.join("good.bin");
        std::fs::write(&good, b"hello").unwrap();
        // sha1("hello")
        let hash = "aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d".to_string();
        let mk = |dest: PathBuf, sha1: &str, size| Job { url: String::new(), dest, sha1: Some(sha1.into()), size: Some(size) };
        let jobs = vec![
            mk(good.clone(), &hash, 5),
            mk(dir.join("missing.bin"), &hash, 5),
            mk(good.clone(), "deadbeef", 5),
        ];
        // Same dest is deduplicated, so only the missing file remains for the first entry's dest.
        let deep = pending(jobs.clone(), true).await.unwrap();
        assert_eq!(deep.len(), 1);
        assert!(deep[0].dest.ends_with("missing.bin"));
        let bad = vec![mk(good.clone(), "deadbeef", 5)];
        assert_eq!(pending(bad.clone(), true).await.unwrap().len(), 1);
        assert_eq!(pending(bad, false).await.unwrap().len(), 0);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
