<#
.SYNOPSIS
    Установка caps-switch в одну команду.

.DESCRIPTION
    Скачивает последний релиз с GitHub, кладёт exe в %LOCALAPPDATA%\Programs\caps-switch,
    настраивает автозапуск и запускает приложение.

    Без параметров — обычный автозапуск (права пользователя).
    -Elevated     — задача Планировщика с наивысшими правами (работает и в админских окнах).
    -Uninstall    — убрать автозапуск, остановить процесс и удалить файлы.

.EXAMPLE
    irm https://raw.githubusercontent.com/svnikolaev/caps-switch/master/install.ps1 | iex

.EXAMPLE
    .\install.ps1 -Elevated

.NOTES
    Требуется Windows 10/11. PowerShell 5.1 или новее.
#>

[CmdletBinding()]
param(
    [switch]$Elevated,
    [switch]$Uninstall,
    [switch]$NoStart,
    [string]$Version = 'latest'
)

$ErrorActionPreference = 'Stop'

[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

$repo = 'svnikolaev/caps-switch'
$installDir = Join-Path $env:LOCALAPPDATA 'Programs\caps-switch'
$exePath = Join-Path $installDir 'caps-switch.exe'

function Write-Step([string]$text) {
    Write-Host "==> $text" -ForegroundColor Cyan
}

function Get-ReleaseAsset {
    param([string]$VersionTag)

    $apiUrl = if ($VersionTag -eq 'latest') {
        "https://api.github.com/repos/$repo/releases/latest"
    }
    else {
        "https://api.github.com/repos/$repo/releases/tags/$VersionTag"
    }

    $release = Invoke-RestMethod -Uri $apiUrl -Headers @{ 'User-Agent' = 'caps-switch-installer' }
    $asset = $release.assets | Where-Object { $_.name -eq 'caps-switch.exe' } | Select-Object -First 1
    if (-not $asset) {
        throw "В релизе $($release.tag_name) нет файла caps-switch.exe"
    }

    [pscustomobject]@{
        Tag  = $release.tag_name
        Url  = $asset.browser_download_url
        Size = $asset.size
    }
}

if ($Uninstall) {
    Write-Step 'Удаление caps-switch'

    if (Test-Path -LiteralPath $exePath) {
        & $exePath --uninstall
    }

    Get-Process caps-switch -ErrorAction SilentlyContinue | Stop-Process -Force
    Start-Sleep -Milliseconds 500

    if (Test-Path -LiteralPath $installDir) {
        Remove-Item -LiteralPath $installDir -Recurse -Force
        Write-Host "Удалена папка: $installDir"
    }

    Write-Host 'Готово. Настройки в HKCU\Software\caps-switch оставлены (удалите вручную при желании).'
    return
}

$asset = Get-ReleaseAsset -VersionTag $Version
Write-Step "Скачивание caps-switch $($asset.Tag)"
Write-Host "    $($asset.Url)"

New-Item -ItemType Directory -Path $installDir -Force | Out-Null

$tmp = Join-Path $env:TEMP ("caps-switch-" + [Guid]::NewGuid().ToString('N') + '.exe')
try {
    Invoke-WebRequest -Uri $asset.Url -OutFile $tmp -UseBasicParsing
    Write-Host "    размер: $([math]::Round((Get-Item $tmp).Length / 1KB)) КБ"

    Get-Process caps-switch -ErrorAction SilentlyContinue | Stop-Process -Force
    Start-Sleep -Milliseconds 500

    Move-Item -LiteralPath $tmp -Destination $exePath -Force
    Write-Host "    установлено: $exePath"
}
finally {
    if (Test-Path -LiteralPath $tmp) { Remove-Item -LiteralPath $tmp -Force -ErrorAction SilentlyContinue }
}

Write-Step 'Настройка автозапуска'
if ($Elevated) {
    $isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
    if (-not $isAdmin) {
        Write-Host '    Нужны права администратора — запускаю exe с повышением (подтверди UAC)...' -ForegroundColor Yellow
        Start-Process -FilePath $exePath -ArgumentList '--install', '--elevated' -Verb RunAs -Wait
    }
    else {
        & $exePath --install --elevated
    }
}
else {
    & $exePath --install
}

if (-not $NoStart) {
    Write-Step 'Запуск'
    Start-Process -FilePath $exePath

    $deadline = (Get-Date).AddSeconds(6)
    $running = $null
    while (-not $running -and (Get-Date) -lt $deadline) {
        Start-Sleep -Milliseconds 300
        $running = Get-Process caps-switch -ErrorAction SilentlyContinue | Select-Object -First 1
    }

    if ($running) {
        Write-Host "    работает (PID $($running.Id))"
    }
    else {
        Write-Host '    процесс не поднялся — проверьте антивирус' -ForegroundColor Yellow
    }
}

Write-Step 'Готово'
Write-Host '    CapsLock — смена раскладки, Shift+CapsLock — обычный Caps.'
Write-Host '    Меню — правый клик по иконке в трее.'
Write-Host '    Удалить: .\install.ps1 -Uninstall'
