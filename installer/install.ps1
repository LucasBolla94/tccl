# Installs the tccl command-line tool on Windows (x86_64) for the current user.
#   powershell -ExecutionPolicy Bypass -c "irm https://tccl.the-coin.cloud/install.ps1 | iex"
# Options: $env:TCCL_VERSION = "v0.3.0"
# The archive is verified against SHA256SUMS published with the same release.
$ErrorActionPreference = "Stop"
$Repo = "LucasBolla94/tccl"
$Version = if ($env:TCCL_VERSION) { $env:TCCL_VERSION } else { "latest" }
$Base = if ($Version -eq "latest") { "https://github.com/$Repo/releases/latest/download" } else { "https://github.com/$Repo/releases/download/$Version" }
$Archive = "tccl-x86_64-pc-windows-msvc.zip"
if (-not [Environment]::Is64BitOperatingSystem) { throw "tccl for Windows needs a 64-bit system" }

$Tmp = Join-Path ([IO.Path]::GetTempPath()) ("tccl-" + [Guid]::NewGuid())
New-Item -ItemType Directory -Path $Tmp | Out-Null
try {
  [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
  Write-Host "Downloading $Archive ($Version)..."
  Invoke-WebRequest -UseBasicParsing -Uri "$Base/$Archive" -OutFile (Join-Path $Tmp $Archive)
  Invoke-WebRequest -UseBasicParsing -Uri "$Base/SHA256SUMS" -OutFile (Join-Path $Tmp "SHA256SUMS")
  $Line = Get-Content (Join-Path $Tmp "SHA256SUMS") | Where-Object { $_ -match " $([Regex]::Escape($Archive))$" } | Select-Object -First 1
  if (-not $Line) { throw "$Archive is not listed in SHA256SUMS" }
  $Expected = ($Line -split " ")[0].ToLower()
  $Actual = (Get-FileHash -Algorithm SHA256 (Join-Path $Tmp $Archive)).Hash.ToLower()
  if ($Expected -ne $Actual) { throw "checksum mismatch for $Archive (expected $Expected, got $Actual)" }

  $Dir = Join-Path $env:LOCALAPPDATA "Programs\tccl"
  New-Item -ItemType Directory -Force -Path $Dir | Out-Null
  Expand-Archive -Force -Path (Join-Path $Tmp $Archive) -DestinationPath $Tmp
  Copy-Item -Force (Join-Path $Tmp "tccl.exe") (Join-Path $Dir "tccl.exe")

  $UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
  if (-not ($UserPath -split ";" | Where-Object { $_ -eq $Dir })) {
    [Environment]::SetEnvironmentVariable("Path", ($(if ($UserPath) { "$UserPath;" } else { "" }) + $Dir), "User")
    Write-Host "Added $Dir to your user PATH (open a new terminal)."
  }
  & (Join-Path $Dir "tccl.exe") version
  Write-Host "Next: tccl new my-first-contract; cd my-first-contract; tccl test"
} finally {
  Remove-Item -Recurse -Force $Tmp -ErrorAction SilentlyContinue
}
