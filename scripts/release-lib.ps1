# Fonctions pures de release.ps1 (testees par release.tests.ps1).

# Zeros de tete refuses : 01.2.3 est invalide.
function Test-Version([string]$V) {
    return [bool]($V -cmatch '\A(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)\z')
}

# -1, 0 ou 1, comparaison numerique composant par composant.
function Compare-Version([string]$A, [string]$B) {
    $pa = $A.Split("."); $pb = $B.Split(".")
    for ($i = 0; $i -lt 3; $i++) {
        $x = [int]$pa[$i]; $y = [int]$pb[$i]
        if ($x -lt $y) { return -1 }
        if ($x -gt $y) { return 1 }
    }
    return 0
}

$script:Utf8NoBom = New-Object Text.UTF8Encoding $false

# Remplace seulement la 1re occurrence : [package] est la 1re section des Cargo.toml.
function Set-FirstMatch([string]$Path, [string]$Pattern, [string]$Version) {
    $text = [IO.File]::ReadAllText($Path, $script:Utf8NoBom)
    $re = New-Object Text.RegularExpressions.Regex($Pattern)
    if (-not $re.IsMatch($text)) { throw "Version introuvable dans $Path" }
    $new = $re.Replace($text, '${1}' + $Version + '${2}', 1)
    [IO.File]::WriteAllText($Path, $new, $script:Utf8NoBom)
}

function Set-Versions([string]$Root, [string]$Version) {
    Set-FirstMatch (Join-Path $Root 'ui\tauri.conf.json') '(?m)^(\s*"version"\s*:\s*")[^"]*(")' $Version
    foreach ($f in 'ui\Cargo.toml', 'engine\Cargo.toml') {
        Set-FirstMatch (Join-Path $Root $f) '(?m)(^\[package\]\r?\n(?:(?!\[)[^\r\n]*\r?\n)*?version\s*=\s*")[^"]*(")' $Version
    }
}

function Get-CurrentVersion([string]$Root) {
    return ([IO.File]::ReadAllText((Join-Path $Root 'ui\tauri.conf.json'), $script:Utf8NoBom) | ConvertFrom-Json).version
}

function New-LatestJson([string]$Version, [string]$Notes, [string]$Signature, [string]$Url, [datetime]$Date) {
    $o = [ordered]@{
        version   = $Version
        notes     = $Notes
        pub_date  = $Date.ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
        platforms = [ordered]@{ 'windows-x86_64' = [ordered]@{ signature = $Signature.Trim(); url = $Url } }
    }
    return ($o | ConvertTo-Json -Depth 5)
}
