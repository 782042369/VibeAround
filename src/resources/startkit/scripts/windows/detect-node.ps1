$ErrorActionPreference = "Stop"

function Emit($obj) {
  $obj | ConvertTo-Json -Compress
}

function Major($version) {
  if (-not $version) { return 0 }
  return [int](($version.TrimStart("v") -split "\.")[0])
}

$minMajor = Major($env:STARTKIT_MIN_VERSION)
$candidate = $null
$cmd = Get-Command node -ErrorAction SilentlyContinue
if ($cmd) { $candidate = $cmd.Source }

if (-not $candidate) {
  $privateNode = Join-Path $env:STARTKIT_NODE_DIR "node.exe"
  if (Test-Path $privateNode) { $candidate = $privateNode }
}

if (-not $candidate) {
  Emit @{ status = "missing"; message = "Node.js $env:STARTKIT_MIN_VERSION or newer will be installed for VibeAround plugins."; actions = @("install") }
  exit 0
}

$version = & $candidate --version 2>$null
if ((Major $version) -lt $minMajor) {
  Emit @{ status = "outdated"; version = $version; path = $candidate; message = "Node.js $version is below the required version. VibeAround will install Node.js $env:STARTKIT_MIN_VERSION or newer for plugins."; actions = @("install") }
  exit 0
}

Emit @{ status = "ok"; version = $version; path = $candidate; message = "Node.js is ready"; actions = @() }
