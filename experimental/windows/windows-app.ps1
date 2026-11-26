# Build a portable Windows bundle, ZIP and WiX MSI for Aether.
# Run from any shell with PowerShell 7+: pwsh experimental/windows/windows-app.ps1
param(
  [switch]$SkipPython,
  [switch]$RequireSigned
)

$ErrorActionPreference = 'Stop'
$ROOT = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
if ($RequireSigned) { $env:AETHER_REQUIRE_SIGNED = '1' }

Write-Host '==> building canonical CLI (release)'
Push-Location $ROOT
cargo build --release -p aether_cli | Out-Null
Write-Host '==> building desktop shell (release)'
cargo build --release --manifest-path experimental/desktop/Cargo.toml | Out-Null
Pop-Location

$OUT = Join-Path $ROOT 'dist/windows'
$BUNDLE = Join-Path $OUT 'Aether'
if (Test-Path $OUT) { Remove-Item -Recurse -Force $OUT }
New-Item -ItemType Directory -Force -Path $BUNDLE, (Join-Path $BUNDLE 'bin') | Out-Null

Write-Host '==> staging resources'
bash (Join-Path $ROOT 'scripts/package/stage-resources.sh') (Join-Path $BUNDLE 'share') (Join-Path $ROOT 'target/release/aether.exe')

if (-not $SkipPython -and $env:AETHER_SKIP_PYTHON -ne '1') {
  Write-Host '==> installing bundled Python runtime'
  $PYSTAGE = Join-Path $OUT 'python-stage'
  bash (Join-Path $ROOT 'scripts/package/fetch-python.sh') $PYSTAGE
  Copy-Item -Recurse -Force $PYSTAGE (Join-Path $BUNDLE 'python')
  $PY = Join-Path $BUNDLE 'python/python.exe'
  if (Test-Path $PY) {
    & $PY --version
    $WHEELS = Join-Path $ROOT 'vendor/wheels'
    if (Test-Path $WHEELS) {
      $SITE = Join-Path $BUNDLE 'python/Lib/site-packages'
      New-Item -ItemType Directory -Force -Path $SITE | Out-Null
      & $PY -m pip install --no-index --find-links $WHEELS --target $SITE --requirement (Join-Path $ROOT 'experimental/desktop/python-runtime.txt')
    } else {
      Write-Host 'NOTE: vendor/wheels absent; runtime wheels are optional'
    }
  }
} else {
  Write-Host 'NOTE: Python bundle skipped'
}

Copy-Item (Join-Path $ROOT 'experimental/desktop/target/release/aether-desktop.exe') (Join-Path $BUNDLE 'Aether.exe')

Write-Host '==> smoke test'
$SMOKE = & (Join-Path $BUNDLE 'Aether.exe') --smoke-test --project $ROOT | Out-String
$JSON = $SMOKE | ConvertFrom-Json
if (-not $JSON.schema.node_types) { throw 'bundle smoke failed' }
Write-Host ("bundle smoke: {0} node types" -f $JSON.schema.node_types.Count)

Write-Host '==> archiving ZIP'
$ZIP = Join-Path $OUT 'Aether-windows.zip'
Compress-Archive -Path $BUNDLE -DestinationPath $ZIP -Force

Write-Host '==> building WiX MSI'
$ICON_PNG = Join-Path $ROOT 'experimental/desktop/AppIcon.png'
$ICON_ICO = Join-Path $OUT 'AppIcon.ico'
if (Test-Path $ICON_PNG) {
  python (Join-Path $ROOT 'experimental/windows/make-ico.py') $ICON_PNG $ICON_ICO | Out-Null
}
$WIX = $env:WIX
$HEAT = if ($WIX) { Join-Path $WIX 'heat.exe' } else { 'heat.exe' }
$CANDLE = if ($WIX) { Join-Path $WIX 'candle.exe' } else { 'candle.exe' }
$LIGHT = if ($WIX) { Join-Path $WIX 'light.exe' } else { 'light.exe' }
if ((Get-Command $HEAT -ErrorAction SilentlyContinue) -and
    (Get-Command $CANDLE -ErrorAction SilentlyContinue) -and
    (Get-Command $LIGHT -ErrorAction SilentlyContinue)) {
  $WXS = Join-Path $OUT 'AetherFiles.wxs'
  & $HEAT dir $BUNDLE -cg AetherFiles -gg -scom -sreg -sfrag -dr INSTALLFOLDER -out $WXS | Out-Null
  $APPWXS = Join-Path $OUT 'Aether.wxs'
  @'
<?xml version="1.0" encoding="utf-8"?>
<Wix xmlns="http://schemas.microsoft.com/wix/2006/wi">
  <Product Id="*" Name="Aether" Language="1033" Version="3.0.0"
           Manufacturer="Crocell Company"
           UpgradeCode="8F3B1D2E-6A4C-4F0B-9D1E-2C5A7B9E4F11">
    <Package InstallerVersion="200" Compressed="yes" InstallScope="perMachine" />
    <Icon Id="AppIcon" SourceFile="AppIcon.ico" />
    <Property Id="ARPPRODUCTICON" Value="AppIcon" />
    <MajorUpgrade DowngradeErrorMessage="An older Aether is installed." />
    <MediaTemplate EmbedCab="yes" />
    <Feature Id="Main" Title="Aether" Level="1">
      <ComponentGroupRef Id="AetherFiles" />
    </Feature>
    <Directory Id="TARGETDIR" Name="SourceDir">
      <Directory Id="ProgramFiles64Folder">
        <Directory Id="INSTALLFOLDER" Name="Aether" />
      </Directory>
    </Directory>
  </Product>
</Wix>
'@ | Set-Content -Path $APPWXS -Encoding UTF8
  Push-Location $OUT
  & $CANDLE -dVERSION=3.0.0 $APPWXS $WXS | Out-Null
  $MSI = Join-Path $OUT 'Aether.msi'
  & $LIGHT -ext WixUIExtension -out $MSI Aether.wixobj AetherFiles.wixobj | Out-Null
  Pop-Location
  Write-Host "==> MSI: $MSI"
} elseif ($env:AETHER_REQUIRE_MSI -eq '1') {
  throw 'WiX toolset required but not found (set WIX)'
} else {
  Write-Host 'NOTE: WiX toolset absent; ZIP produced without MSI'
}

$artifacts = @($ZIP)
if (Test-Path (Join-Path $OUT 'Aether.msi')) { $artifacts += (Join-Path $OUT 'Aether.msi') }
foreach ($artifact in $artifacts) {
  if (Get-Command bash -ErrorAction SilentlyContinue) {
    bash (Join-Path $ROOT 'scripts/package/sign-artifacts.sh') $artifact.Replace('\','/')
  }
}
Write-Host "Built $BUNDLE"
