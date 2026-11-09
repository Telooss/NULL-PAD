# Tests simples (Pester 3.4 est trop vieux). Sortie non nulle si un test echoue.
$ErrorActionPreference = 'Stop'
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
. (Join-Path $here 'release-lib.ps1')
$repo = Split-Path -Parent $here
$script:fail = 0
function Check($name, $ok) {
    if ($ok) { Write-Host "ok   $name" } else { Write-Host "FAIL $name"; $script:fail++ }
}

# Test-Version : les zeros de tete sont refuses (01.2.3 invalide)
foreach ($v in '0.1.0', '1.2.3', '10.20.30') { Check "Test-Version valide $v" ((Test-Version $v) -eq $true) }
foreach ($v in '1.2', 'v1.2.3', '1.2.3-beta', '01.2.3', '1.2.3.4', '', '1.2.3 ', "1.2.3`n") { Check "Test-Version invalide '$v'" ((Test-Version $v) -eq $false) }

# Compare-Version : numerique
Check 'Compare 0.10.0 > 0.9.0' ((Compare-Version '0.10.0' '0.9.0') -eq 1)
Check 'Compare 0.9.0 < 0.10.0' ((Compare-Version '0.9.0' '0.10.0') -eq -1)
Check 'Compare egal' ((Compare-Version '1.2.3' '1.2.3') -eq 0)
Check 'Compare major' ((Compare-Version '2.0.0' '1.99.99') -eq 1)
Check 'Compare patch' ((Compare-Version '1.2.3' '1.2.4') -eq -1)

# Set-Versions sur copies temporaires des vrais fichiers
$tmp = Join-Path ([IO.Path]::GetTempPath()) ("relt-" + [guid]::NewGuid())
New-Item -ItemType Directory "$tmp\ui", "$tmp\engine" | Out-Null
$files = 'ui\tauri.conf.json', 'ui\Cargo.toml', 'engine\Cargo.toml'
foreach ($f in $files) { Copy-Item (Join-Path $repo $f) (Join-Path $tmp $f) }
$before = @{}; foreach ($f in $files) { $before[$f] = [IO.File]::ReadAllBytes((Join-Path $tmp $f)) }
Set-Versions -Root $tmp -Version '9.8.7'
$enc = New-Object Text.UTF8Encoding $false
foreach ($f in $files) {
    $old = $enc.GetString($before[$f]) -split '\n'
    $new = $enc.GetString([IO.File]::ReadAllBytes((Join-Path $tmp $f))) -split '\n'
    $diff = @(0..($old.Count - 1) | Where-Object { $old[$_] -ne $new[$_] })
    Check "$f : meme nombre de lignes" ($old.Count -eq $new.Count)
    Check "$f : une seule ligne changee" ($diff.Count -eq 1)
    Check "$f : nouvelle version" ($new[$diff[0]] -match '9\.8\.7')
    $raw = [IO.File]::ReadAllBytes((Join-Path $tmp $f))
    Check "$f : pas de BOM" (-not ($raw.Length -ge 3 -and $raw[0] -eq 0xEF -and $raw[1] -eq 0xBB))
    $cnt = { param($b) $s = $enc.GetString($b); '{0},{1}' -f ([regex]::Matches($s, '\r\n')).Count, ([regex]::Matches($s, '(?<!\r)\n')).Count }
    Check "$f : fins de ligne preservees" ((& $cnt $raw) -eq (& $cnt $before[$f]))
}
$ui = [IO.File]::ReadAllText((Join-Path $tmp 'ui\Cargo.toml'))
Check 'ui : tauri intact' ($ui.Contains('tauri = { version = "2.12", features = [] }'))
Check 'ui : prost intact' ($ui.Contains('prost = "0.14"'))
Check 'ui : prost-build intact' ($ui.Contains('prost-build = "0.14"'))
$json = [IO.File]::ReadAllText((Join-Path $tmp 'ui\tauri.conf.json')) | ConvertFrom-Json
Check 'tauri.conf.json valide et version' ($json.version -eq '9.8.7')
Check 'Get-CurrentVersion' ((Get-CurrentVersion -Root $tmp) -eq '9.8.7')
Remove-Item $tmp -Recurse -Force

# New-LatestJson
$sig = "dW50cnVzdGVkIGNvbW1lbnQ6IHg=`nUlVRU...abc=="
$url = 'https://github.com/Telooss/NULL-PAD/releases/download/v1.2.3/NULL-CONTROL_1.2.3_x64-setup.exe'
$j = New-LatestJson -Version '1.2.3' -Notes 'Un "test"' -Signature $sig -Url $url -Date ([datetime]'2026-11-09T10:11:12Z').ToUniversalTime()
$o = $j | ConvertFrom-Json
Check 'json version' ($o.version -eq '1.2.3')
Check 'json notes' ($o.notes -eq 'Un "test"')
Check 'json pub_date' ($o.pub_date -eq '2026-11-09T10:11:12Z')
Check 'json signature verbatim' ($o.platforms.'windows-x86_64'.signature -eq $sig)
Check 'json url' ($o.platforms.'windows-x86_64'.url -eq $url)
Check 'json newline echappe' ($j.Contains('\n') -and -not ($j -match '"signature":\s*"[^"]*\r?\n'))
Check 'json une seule plateforme' (@($o.platforms.PSObject.Properties).Count -eq 1)

if ($script:fail) { Write-Host "$script:fail echec(s)"; exit 1 }
Write-Host 'Tout est vert'
