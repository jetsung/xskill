#============================================================
# File: install.ps1
# Description: XSkill 安装脚本 (Windows)
# URL: https://xskill.gcli.cn/install.ps1
# Author: Jetsung Chan <i@jetsung.com>
# Version: 0.1.0
# CreatedAt: 2026-07-20
# UpdatedAt: 2026-07-20
#============================================================

[CmdletBinding()]
param(
    [switch]$Pre,
    [string]$Url
)

$ErrorActionPreference = "Stop"

$CDN_URL = if ($env:CDN) { $env:CDN } else { "https://fastfile.asfd.cn/" }

# 检测是否为管理员
function Test-Administrator {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = New-Object Security.Principal.WindowsPrincipal($identity)
    return $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

function Test-Command {
    param([string]$Name)
    return [bool](Get-Command $Name -ErrorAction SilentlyContinue)
}

function Test-ChinaNetwork {
    if ($env:CN) {
        return $true  # 手动指定
    }
    try {
        $request = [System.Net.WebRequest]::Create("https://www.google.com")
        $request.Timeout = 3000
        $response = $request.GetResponse()
        $response.Close()
        return $false  # 非中国网络
    } catch {
        return $true   # 中国网络
    }
}

# 若为 https://xxx.xx 不以 / 结尾，则组合时去掉加速网址的 https://
#   格式为 https://file.xxx.io/github.com/
# 若为 https://xxx.xx/ 以 / 结尾，则组合时保留加速网址的 https://
#   格式为 https://xxx.xx/https://github.com/
function Get-RemoveHttpsFlag {
    param([string]$CdnUrl)
    if ($CdnUrl -and -not $CdnUrl.EndsWith("/")) {
        return $true
    }
    return $false
}

function Remove-SecondHttps {
    param(
        [string]$Url,
        [bool]$ShouldRemove
    )
    if ($ShouldRemove) {
        $idx = $Url.IndexOf("https://")
        if ($idx -ge 0) {
            $secondIdx = $Url.IndexOf("https://", $idx + 1)
            if ($secondIdx -ge 0) {
                return $Url.Substring(0, $secondIdx) + $Url.Substring($secondIdx + 8)
            }
        }
    }
    return $Url
}

########################## 以上为通用函数 #########################

# xskill 资产命名规则（release workflow）：
#   xskill-v{VERSION}-{arch}-pc-windows-msvc.zip
#   arch: x86_64 | aarch64
function Get-DownloadUrl {
    param(
        [string]$Repo,
        [string]$Arch,
        [bool]$IsPre,
        [string]$CdnUrl,
        [bool]$NoHttps
    )

    $apiUrl = Remove-SecondHttps "$($CdnUrl)https://api.github.com/repos/$Repo/releases" $NoHttps

    if (-not $IsPre) {
        $apiUrl = "$apiUrl/latest"
    }

    $headers = @{ "User-Agent" = "xskill-installer" }

    if ($IsPre) {
        $releases = Invoke-RestMethod -Uri $apiUrl -Headers $headers
        $release = $releases | Where-Object { $_.prerelease -eq $true } | Select-Object -First 1
        if (-not $release) {
            Write-Error "No pre-release found."
            exit 1
        }
        $asset = $release.assets | Where-Object {
            $_.name -match "$Arch-pc-windows-msvc" -and $_.name -match '\.zip$'
        } | Select-Object -First 1
    } else {
        $release = Invoke-RestMethod -Uri $apiUrl -Headers $headers
        $asset = $release.assets | Where-Object {
            $_.name -match "$Arch-pc-windows-msvc" -and $_.name -match '\.zip$'
        } | Select-Object -First 1
    }

    if (-not $asset) {
        Write-Error "No matching asset found for arch=$Arch."
        exit 1
    }

    return $asset.browser_download_url
}

function Install-XSkill {
    param(
        [string]$DownloadUrl,
        [string]$CdnUrl,
        [bool]$NoHttps,
        [string]$BinPath
    )

    $tmpDir = Join-Path ([System.IO.Path]::GetTempPath()) "xskill_$([System.IO.Path]::GetRandomFileName())"
    New-Item -ItemType Directory -Path $tmpDir -Force | Out-Null

    try {
        $downloadFile = Join-Path $tmpDir "xskill.zip"

        $downloadTarget = Remove-SecondHttps "$($CdnUrl)$DownloadUrl" $NoHttps

        Write-Host "Downloading $downloadTarget ..."
        $headers = @{ "User-Agent" = "xskill-installer" }
        Invoke-WebRequest -Uri $downloadTarget -OutFile $downloadFile -Headers $headers

        Write-Host "Extracting ..."
        Expand-Archive -Path $downloadFile -DestinationPath $tmpDir -Force

        $exePath = Join-Path $tmpDir "xskill.exe"
        if (-not (Test-Path $exePath)) {
            Write-Error "Error: xskill.exe not found in archive."
            exit 1
        }

        New-Item -ItemType Directory -Path $BinPath -Force | Out-Null
        Copy-Item -Path $exePath -Destination $BinPath -Force
    } finally {
        Remove-Item -Path $tmpDir -Recurse -Force -ErrorAction SilentlyContinue
    }
}

function Main {
    # 优先级：命令行参数 > 环境变量 > 默认流程
    $customUrl = $Url
    if (-not $customUrl -and $env:URL) {
        $customUrl = $env:URL
    }
    $downloadUrl = if ($customUrl) { $customUrl } else { $env:URL }

    # 架构检测
    $arch = switch ($env:PROCESSOR_ARCHITECTURE) {
        "AMD64"   { "x86_64" }
        "ARM64"   { "aarch64" }
        default   {
            Write-Error "Unsupported architecture: $env:PROCESSOR_ARCHITECTURE"
            exit 1
        }
    }

    if (-not $downloadUrl) {
        $isChina = Test-ChinaNetwork
        if (-not $isChina) {
            $CDN_URL = ""
        }

        $noHttps = Get-RemoveHttpsFlag $CDN_URL

        $downloadUrl = Get-DownloadUrl -Repo "jetsung/xskill" -Arch $arch -IsPre $Pre -CdnUrl $CDN_URL -NoHttps $noHttps
    } else {
        Write-Host "Using specified download URL: $downloadUrl"
        $noHttps = $false
    }

    # 安装路径
    if (Test-Administrator) {
        $binPath = Join-Path $env:ProgramFiles "xskill\bin"
    } else {
        $binPath = Join-Path $env:USERPROFILE ".local\bin"
    }

    Install-XSkill -DownloadUrl $downloadUrl -CdnUrl $CDN_URL -NoHttps $noHttps -BinPath $binPath

    Write-Host ""

    $exePath = Join-Path $binPath "xskill.exe"
    if (-not (Test-Path $exePath)) {
        Write-Host "xskill has not been installed successfully."
        Write-Host ""
        exit 1
    }

    # 检查 PATH
    $pathDirs = $env:PATH -split ";"
    if ($binPath -notin $pathDirs) {
        Write-Host ""
        Write-Host "NOTE: '$binPath' is not in your PATH."
        Write-Host "Add it by running:"
        Write-Host ""
        Write-Host "  [Environment]::SetEnvironmentVariable('PATH', `"$binPath;`$env:PATH`", 'User')"
        Write-Host ""
    }

    Write-Host ""
    Write-Host "xskill has been installed successfully!"
    Write-Host ""
    & $exePath --help
    Write-Host ""
    & $exePath --version
    Write-Host ""
}

Main
