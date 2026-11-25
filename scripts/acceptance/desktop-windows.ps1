# Acceptance: Windows desktop bundle boots the embedded console.
$ErrorActionPreference = 'Stop'
$ROOT = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$BUNDLE = Join-Path $ROOT 'dist/windows/Aether'
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"

function Fail($message) { throw "FAIL: $message" }
function Pass($message) { Write-Host "PASS: $message" }

Write-Host '== 1. Windows bundle exists =='
if (-not (Test-Path (Join-Path $BUNDLE 'Aether.exe'))) {
  pwsh -NoProfile -File (Join-Path $ROOT 'scripts/package/windows-app.ps1') -SkipPython | Out-Null
}
foreach ($path in @('Aether.exe', 'bin/aether.exe', 'share/aether/contracts')) {
  if (-not (Test-Path (Join-Path $BUNDLE $path))) { Fail "missing $path" }
}
Pass 'bundle contains shell, CLI and contracts'

Write-Host '== 2. smoke test =='
$OUT = & (Join-Path $BUNDLE 'Aether.exe') --smoke-test --project $ROOT | Out-String
$JSON = $OUT | ConvertFrom-Json
if ($JSON.schema.node_types.Count -ne 78) { Fail 'expected 78 node types' }
if ($JSON.status.certification_claim -ne $false) { Fail 'certification claim must be false' }
Pass 'in-process server serves the canonical atlas'

Write-Host '== 3. no token is persisted =='
if (Get-ChildItem -Recurse -Path $BUNDLE -Filter '*token*') { Fail 'token material found in bundle' }
Pass 'per-launch token stays in memory'

Write-Host ''
Write-Host 'Windows desktop acceptance complete. CLI remains untouched.'
