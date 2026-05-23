param(
    [string]$AppName = ""
)

$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$projectRoot = Resolve-Path (Join-Path $scriptDir "..")

if ([string]::IsNullOrWhiteSpace($AppName)) {
    $tauriConfigPath = Join-Path $projectRoot "src-tauri\tauri.conf.json"
    $tauriConfig = Get-Content -LiteralPath $tauriConfigPath -Raw | ConvertFrom-Json
    $AppName = [string]$tauriConfig.productName
}

if ([string]::IsNullOrWhiteSpace($AppName)) {
    throw "Unable to determine app name for MSI install root patch."
}

$wixRoot = Join-Path $projectRoot "src-tauri\target\release\wix"
$wxs = Get-ChildItem -LiteralPath $wixRoot -Recurse -Filter "main.wxs" -ErrorAction SilentlyContinue |
    Sort-Object LastWriteTime -Descending |
    Select-Object -First 1

if ($null -eq $wxs) {
    throw "WiX output was not found. Run tauri build first."
}

$wixDir = $wxs.DirectoryName
$arch = Split-Path -Leaf $wixDir
$content = Get-Content -LiteralPath $wxs.FullName -Raw
$escapedAppName = [regex]::Escape($AppName)
$directoryPattern = '<Directory Id="\$\(var\.PlatformProgramFilesFolder\)" Name="PFiles">\s*<Directory Id="INSTALLDIR" Name="' + $escapedAppName + '"\s*/>\s*</Directory>'
$reliquaryDirectory = @"
<Directory Id="`$(var.PlatformProgramFilesFolder)" Name="PFiles">
                <Directory Id="RELIQUARYDIR" Name="Reliquary">
                    <Directory Id="INSTALLDIR" Name="$AppName"/>
                </Directory>
            </Directory>
"@

if ($content -notmatch '<Directory Id="RELIQUARYDIR" Name="Reliquary">') {
    $patched = [regex]::Replace($content, $directoryPattern, $reliquaryDirectory, 1)

    if ($patched -eq $content) {
        throw "Unable to patch WiX INSTALLDIR for $AppName."
    }

    $utf8NoBom = New-Object System.Text.UTF8Encoding -ArgumentList $false
    [System.IO.File]::WriteAllText($wxs.FullName, $patched, $utf8NoBom)
}

$wixTools = Join-Path $env:LOCALAPPDATA "tauri\WixTools314"
$candle = Join-Path $wixTools "candle.exe"
$light = Join-Path $wixTools "light.exe"

if (!(Test-Path -LiteralPath $candle) -or !(Test-Path -LiteralPath $light)) {
    throw "WiX tools were not found in $wixTools."
}

$msiRoot = Join-Path $projectRoot "src-tauri\target\release\bundle\msi"
$msi = Get-ChildItem -LiteralPath $msiRoot -Filter "*.msi" -ErrorAction SilentlyContinue |
    Sort-Object LastWriteTime -Descending |
    Select-Object -First 1

if ($null -eq $msi) {
    throw "MSI output was not found in $msiRoot."
}

$objPath = Join-Path $wixDir "main.wixobj"
$localePath = Join-Path $wixDir "locale.wxl"

& $candle -nologo -arch $arch -out $objPath $wxs.FullName
if ($LASTEXITCODE -ne 0) {
    throw "WiX candle failed while rebuilding $($msi.Name)."
}

$lightArgs = @("-nologo", "-ext", "WixUIExtension", "-cultures:en-US")
if (Test-Path -LiteralPath $localePath) {
    $lightArgs += @("-loc", $localePath)
}
$lightArgs += @("-out", $msi.FullName, $objPath)

& $light @lightArgs
if ($LASTEXITCODE -ne 0) {
    throw "WiX light failed while rebuilding $($msi.Name)."
}

Write-Host "MSI install root patched to Program Files\Reliquary\$AppName"
