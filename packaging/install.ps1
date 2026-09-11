param(
    [Parameter(Mandatory = $false)]
    [string] $Version
)

$ErrorActionPreference = 'Stop'

function Show-Usage {
    @'
Usage: install.ps1 [-Version X.Y.Z]

Install the prebuilt stegoeggo CLI for Windows AMD64. An unsupported target or
missing binary asset may fall back to cargo install stegoeggo-cli --locked.
'@ | Write-Output
}

function Fail([string] $Message) {
    throw "stegoeggo installer: $Message"
}

function Validate-Version([string] $Value) {
    if ($Value -notmatch '^[0-9]+\.[0-9]+\.[0-9]+$') {
        Fail "invalid version '$Value'; expected X.Y.Z"
    }
}

function Invoke-Download([string] $Url, [string] $Destination) {
    try {
        Invoke-WebRequest -Uri $Url -OutFile $Destination -UseBasicParsing
        return 200
    } catch {
        $response = $_.Exception.Response
        if ($null -ne $response) {
            return [int] $response.StatusCode
        }
        return 0
    }
}

function Invoke-CargoFallback([string] $RequestedVersion) {
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        Fail 'no binary is published for this target and Cargo is not installed'
    }
    Write-Warning 'No compatible prebuilt binary was found; falling back to Cargo.'
    if ([string]::IsNullOrEmpty($RequestedVersion)) {
        & cargo install stegoeggo-cli --locked
    } else {
        & cargo install stegoeggo-cli --locked --version $RequestedVersion
    }
    if ($LASTEXITCODE -ne 0) {
        Fail "Cargo fallback failed with exit code $LASTEXITCODE"
    }
}

if ($null -ne $Version) {
    Validate-Version $Version
}

$target = 'x86_64-pc-windows-msvc'
if ($env:PROCESSOR_ARCHITECTURE -ne 'AMD64' -and $env:PROCESSOR_ARCHITEW6432 -ne 'AMD64') {
    Invoke-CargoFallback $Version
    exit 0
}

$asset = "stegoeggo-$target.exe"
$releasesUrl = if ([string]::IsNullOrEmpty($env:STEGOEGGO_RELEASES_URL)) {
    'https://github.com/eggstack/stegoeggo/releases'
} else {
    $env:STEGOEGGO_RELEASES_URL.TrimEnd('/')
}
$baseUrl = if ([string]::IsNullOrEmpty($Version)) {
    "$releasesUrl/latest/download"
} else {
    "$releasesUrl/download/v$Version"
}
$tempDir = Join-Path ([System.IO.Path]::GetTempPath()) ("stegoeggo-install-" + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $tempDir | Out-Null

try {
    $binaryPath = Join-Path $tempDir $asset
    $checksumPath = "$binaryPath.sha256"
    $status = Invoke-Download "$baseUrl/$asset" $binaryPath
    if ($status -eq 404) {
        Invoke-CargoFallback $Version
        exit 0
    }
    if ($status -lt 200 -or $status -ge 300) {
        Fail "failed to download binary asset (HTTP $status)"
    }

    $status = Invoke-Download "$baseUrl/$asset.sha256" $checksumPath
    if ($status -lt 200 -or $status -ge 300) {
        Fail "failed to download checksum sidecar (HTTP $status)"
    }
    $expectedHash = ((Get-Content -Raw $checksumPath) -split '\s+')[0].ToLowerInvariant()
    if ($expectedHash -notmatch '^[0-9a-f]{64}$') {
        Fail 'checksum sidecar does not contain a SHA-256 digest'
    }
    $actualHash = (Get-FileHash -Algorithm SHA256 -Path $binaryPath).Hash.ToLowerInvariant()
    if ($actualHash -ne $expectedHash) {
        Fail 'checksum verification failed'
    }

    $candidateIdentity = & $binaryPath version
    if ($LASTEXITCODE -ne 0 -or $candidateIdentity -notmatch '^stegoeggo ([0-9]+\.[0-9]+\.[0-9]+)$') {
        Fail "candidate version identity is invalid: $candidateIdentity"
    }
    $candidateVersion = $Matches[1]
    if (-not [string]::IsNullOrEmpty($Version) -and $candidateVersion -ne $Version) {
        Fail "candidate version is $candidateVersion, expected $Version"
    }

    $destination = if ([string]::IsNullOrEmpty($env:LOCALAPPDATA)) {
        Join-Path $env:USERPROFILE 'AppData\Local'
    } else {
        $env:LOCALAPPDATA
    }
    $destination = Join-Path $destination 'StegoEggo\bin'
    New-Item -ItemType Directory -Path $destination -Force | Out-Null
    $installedPath = Join-Path $destination 'stegoeggo.exe'
    Copy-Item -Force $binaryPath $installedPath
    Write-Output "Installed stegoeggo $candidateVersion to $installedPath"
    if ($env:PATH -notlike "*${destination}*") {
        Write-Warning "$destination is not on PATH; add it for direct use."
    }
} finally {
    if (Test-Path $tempDir) {
        Remove-Item -Recurse -Force $tempDir
    }
}
