# Rebuilds src-tauri/resources/diamond-agent.jar (Java 8 bytecode, needs a JDK on PATH or JAVA_HOME).
$ErrorActionPreference = "Stop"
$bin = if ($env:JAVA_HOME) { Join-Path $env:JAVA_HOME "bin" } else { "" }
$javac = if ($bin) { Join-Path $bin "javac.exe" } else { "javac" }
$jar = if ($bin) { Join-Path $bin "jar.exe" } else { "jar" }
$out = Join-Path $PSScriptRoot "out"
Remove-Item $out -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory $out | Out-Null
& $javac --release 8 -d $out (Join-Path $PSScriptRoot "src\dev\diamond\BrandAgent.java")
$target = Join-Path $PSScriptRoot "..\src-tauri\resources"
New-Item -ItemType Directory $target -Force | Out-Null
& $jar cfm (Join-Path $target "diamond-agent.jar") (Join-Path $PSScriptRoot "MANIFEST.MF") -C $out .
Remove-Item $out -Recurse -Force
