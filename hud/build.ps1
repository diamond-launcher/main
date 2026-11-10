# Rebuilds src-tauri/resources/diamond-hud.jar.
#
# The mod is compiled against the official-named Minecraft 26.x client jar
# (26.x ships unobfuscated, so no remapping step is needed) plus the Fabric
# Loader and Fabric API rendering module that the launcher already installed.
#
# Usage:
#   .\hud\build.ps1
#   .\hud\build.ps1 -MinecraftVersion 26.3 -GameDir 'D:\minecraft'
param(
    [string]$MinecraftVersion = "26.3",
    [string]$GameDir = (Join-Path $env:APPDATA "Diamond\minecraft"),
    [string]$Jdk = ""
)

$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.IO.Compression.FileSystem

function Find-Jdk {
    param([string]$Preferred)
    if ($Preferred) {
        return $Preferred
    }
    if ($env:JAVA_HOME -and (Test-Path (Join-Path $env:JAVA_HOME "bin\javac.exe"))) {
        $major = & (Join-Path $env:JAVA_HOME "bin\javac.exe") -version 2>&1
        if ("$major" -match '^javac 2[5-9]') {
            return $env:JAVA_HOME
        }
    }
    $adoptium = "C:\Program Files\Eclipse Adoptium"
    if (Test-Path $adoptium) {
        $candidate = Get-ChildItem $adoptium -Directory -Filter "jdk-25*" |
            Sort-Object Name -Descending |
            Select-Object -First 1
        if ($candidate) {
            return $candidate.FullName
        }
    }
    throw "A JDK 25+ is required. Pass -Jdk <path> or set JAVA_HOME."
}

$jdkRoot = Find-Jdk $Jdk
$javac = Join-Path $jdkRoot "bin\javac.exe"
$jarTool = Join-Path $jdkRoot "bin\jar.exe"

$minecraftJar = Join-Path $GameDir "versions\$MinecraftVersion\$MinecraftVersion.jar"
if (-not (Test-Path $minecraftJar)) {
    throw "Minecraft jar not found: $minecraftJar (install $MinecraftVersion with the launcher first)."
}

$loaderJar = Get-ChildItem (Join-Path $GameDir "libraries\net\fabricmc\fabric-loader") -Recurse -Filter "fabric-loader-*.jar" -ErrorAction SilentlyContinue |
    Sort-Object LastWriteTime -Descending |
    Select-Object -First 1 -ExpandProperty FullName
if (-not $loaderJar) {
    throw "Fabric Loader jar not found under $GameDir\libraries\net\fabricmc\fabric-loader."
}

$libraryJars = Get-ChildItem (Join-Path $GameDir "libraries") -Recurse -Filter "*.jar" -ErrorAction SilentlyContinue |
    ForEach-Object { $_.FullName }

$fabricApi = Get-ChildItem (Join-Path $GameDir "diamond-mods\$MinecraftVersion") -Filter "fabric-api-*.jar" -ErrorAction SilentlyContinue |
    Sort-Object LastWriteTime -Descending |
    Select-Object -First 1 -ExpandProperty FullName
if (-not $fabricApi) {
    throw "Fabric API jar not found for $MinecraftVersion. Install that version once with mods enabled."
}

$jomlJar = Get-ChildItem (Join-Path $GameDir "libraries\org\joml\joml") -Recurse -Filter "joml-*.jar" -ErrorAction SilentlyContinue |
    Sort-Object LastWriteTime -Descending |
    Select-Object -First 1 -ExpandProperty FullName

$jspecifyJar = Get-ChildItem (Join-Path $GameDir "libraries\org\jspecify\jspecify") -Recurse -Filter "jspecify-*.jar" -ErrorAction SilentlyContinue |
    Sort-Object LastWriteTime -Descending |
    Select-Object -First 1 -ExpandProperty FullName

$temp = Join-Path $env:TEMP ("diamond-hud-build-" + [guid]::NewGuid().ToString("N"))
$out = Join-Path $PSScriptRoot "out"
New-Item -ItemType Directory $temp -Force | Out-Null
New-Item -ItemType Directory $out -Force | Out-Null

try {
    $renderingJar = Join-Path $temp "fabric-rendering-v1.jar"
    $keyMappingJar = Join-Path $temp "fabric-key-mapping-api-v1.jar"
    $screenJar = Join-Path $temp "fabric-screen-api-v1.jar"
    $lifecycleJar = Join-Path $temp "fabric-lifecycle-events-v1.jar"
    $apiBaseJar = Join-Path $temp "fabric-api-base.jar"
    $zip = [IO.Compression.ZipFile]::OpenRead($fabricApi)
    try {
        $entry = $zip.Entries |
            Where-Object { $_.FullName -match '^META-INF/jars/fabric-rendering-v1-.*\.jar$' } |
            Select-Object -First 1
        if (-not $entry) {
            throw "fabric-rendering-v1 module not found inside $fabricApi."
        }
        [IO.Compression.ZipFileExtensions]::ExtractToFile($entry, $renderingJar, $true)

        $entry = $zip.Entries |
            Where-Object { $_.FullName -match '^META-INF/jars/fabric-key-mapping-api-v1-.*\.jar$' } |
            Select-Object -First 1
        if (-not $entry) {
            throw "fabric-key-mapping-api-v1 module not found inside $fabricApi."
        }
        [IO.Compression.ZipFileExtensions]::ExtractToFile($entry, $keyMappingJar, $true)

        $entry = $zip.Entries |
            Where-Object { $_.FullName -match '^META-INF/jars/fabric-screen-api-v1-.*\.jar$' } |
            Select-Object -First 1
        if (-not $entry) {
            throw "fabric-screen-api-v1 module not found inside $fabricApi."
        }
        [IO.Compression.ZipFileExtensions]::ExtractToFile($entry, $screenJar, $true)

        $entry = $zip.Entries |
            Where-Object { $_.FullName -match '^META-INF/jars/fabric-lifecycle-events-v1-.*\.jar$' } |
            Select-Object -First 1
        if (-not $entry) {
            throw "fabric-lifecycle-events-v1 module not found inside $fabricApi."
        }
        [IO.Compression.ZipFileExtensions]::ExtractToFile($entry, $lifecycleJar, $true)

        $entry = $zip.Entries |
            Where-Object { $_.FullName -match '^META-INF/jars/fabric-api-base-.*\.jar$' } |
            Select-Object -First 1
        if (-not $entry) {
            throw "fabric-api-base module not found inside $fabricApi."
        }
        [IO.Compression.ZipFileExtensions]::ExtractToFile($entry, $apiBaseJar, $true)
    } finally {
        $zip.Dispose()
    }

    $classpath = @($minecraftJar, $renderingJar, $keyMappingJar, $screenJar, $lifecycleJar, $apiBaseJar) + $libraryJars
    if ($jomlJar -and $classpath -notcontains $jomlJar) {
        $classpath += $jomlJar
    }
    if ($jspecifyJar -and $classpath -notcontains $jspecifyJar) {
        $classpath += $jspecifyJar
    }

    $sources = Get-ChildItem (Join-Path $PSScriptRoot "src\main\java") -Recurse -Filter "*.java" |
        ForEach-Object { $_.FullName }

    Write-Host "Compiling Diamond HUD against $MinecraftVersion..."
    & $javac --release 25 -encoding UTF-8 -classpath ($classpath -join ";") -d $out $sources
    if ($LASTEXITCODE -ne 0) {
        throw "javac failed with exit code $LASTEXITCODE."
    }

    Copy-Item (Join-Path $PSScriptRoot "src\main\resources\fabric.mod.json") $out

    $targetDir = Join-Path $PSScriptRoot "..\src-tauri\resources"
    New-Item -ItemType Directory $targetDir -Force | Out-Null
    $target = Join-Path $targetDir "diamond-hud.jar"
    & $jarTool cf $target -C $out .
    if ($LASTEXITCODE -ne 0) {
        throw "jar failed with exit code $LASTEXITCODE."
    }
    Write-Host ("Built {0} ({1:N0} bytes)" -f $target, (Get-Item $target).Length)
} finally {
    Remove-Item $out -Recurse -Force -ErrorAction SilentlyContinue
    Remove-Item $temp -Recurse -Force -ErrorAction SilentlyContinue
}
