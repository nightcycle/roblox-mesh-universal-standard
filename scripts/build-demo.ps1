$ErrorActionPreference = 'Stop'
$previousTemp = $env:TEMP
$previousTmp = $env:TMP
Push-Location (Join-Path $PSScriptRoot '..')
try {
    New-Item -ItemType Directory -Force dist, target/tmp | Out-Null
    $env:TEMP = (Resolve-Path target/tmp).Path
    $env:TMP = $env:TEMP
    cargo build --locked --release
    if ($LASTEXITCODE -ne 0) { throw 'Rust build failed' }
    foreach ($name in @('grass', 'lilypad', 'pillar')) {
        & ./target/release/rmus.exe export "test/$name.rbxm" -o "demo/assets/$name.json" --children "demo/assets/$name.model.json"
        if ($LASTEXITCODE -ne 0) { throw "Export failed: $name" }
    }
    rojo build demo.project.json --output dist/demo.rbxl
    if ($LASTEXITCODE -ne 0) { throw 'Rojo build failed' }
} finally {
    $env:TEMP = $previousTemp
    $env:TMP = $previousTmp
    Pop-Location
}
