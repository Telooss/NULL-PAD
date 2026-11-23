# Publication locale de NULL/CONTROL : scripts\release.ps1 -Version X.Y.Z [-Notes "..."] [-Publish]
param(
    [Parameter(Mandatory = $true)][string]$Version,
    [string]$Notes = '',
    [switch]$Publish
)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'release-lib.ps1')
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

$bumped = $false
function Fail([string]$msg) {
    Write-Host "Erreur : $msg" -ForegroundColor Red
    if ($bumped) { Write-Host 'Pour reessayer : git checkout -- .' }
    exit 1
}
# Lance une commande native et s'arrete si elle echoue.
function Step([string]$label, [scriptblock]$cmd) {
    Write-Host "== $label"
    $old = $ErrorActionPreference; $ErrorActionPreference = 'Continue'  # stderr natif ne doit pas lever
    try { & $cmd } finally { $ErrorActionPreference = $old }
    if ($LASTEXITCODE -ne 0) { Fail "$label a echoue (code $LASTEXITCODE)." }
}

# --- 1. Refus ---
if (-not (Test-Version $Version)) { Fail "Version invalide '$Version' (attendu X.Y.Z)." }
$current = Get-CurrentVersion -Root $root
$cmp = Compare-Version $Version $current
$tags = @(git tag -l "v$current")
# Premiere release : v0.1.0 = version courante, tant qu'aucun tag v<courante> n'existe.
$skipBump = ($cmp -eq 0 -and $tags.Count -eq 0)
if ($cmp -lt 0 -or ($cmp -eq 0 -and -not $skipBump)) { Fail "La version doit etre superieure a $current." }
if (@(git tag -l "v$Version").Count -gt 0) { Fail "Le tag v$Version existe deja." }
if (@(git status --porcelain).Count -gt 0) { Fail "L'arbre git n'est pas propre." }
if ((git branch --show-current) -ne 'main') { Fail "Il faut etre sur la branche main." }
$keyFile = Join-Path $env:USERPROFILE '.nullctl\updater.key'
if (-not (Test-Path $keyFile)) { Fail "Cle absente ($keyFile). Genere-la : cargo tauri signer generate -w `"$keyFile`"" }
$pubkey = (Get-Content (Join-Path $root 'ui\tauri.bundle.json') -Raw | ConvertFrom-Json).plugins.updater.pubkey
if ([string]::IsNullOrWhiteSpace($pubkey)) { Fail "pubkey vide dans ui\tauri.bundle.json : genere la cle (cargo tauri signer generate) et colle la cle publique." }
if ($Publish) {
    # Pas de cmd /c : cmd peut ne pas trouver gh alors que PowerShell le trouve.
    if (-not (Get-Command gh -ErrorAction SilentlyContinue)) { Fail "gh introuvable (installe GitHub CLI)." }
    $ErrorActionPreference = 'Continue'  # stderr natif ne doit pas lever
    gh auth status *> $null
    $ErrorActionPreference = 'Stop'
    if ($LASTEXITCODE -ne 0) { Fail "gh n'est pas connecte (gh auth login)." }
}

# Mot de passe de la cle : jamais affiche ni journalise, limite a ce process et ses enfants.
$pw = $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD
if (-not $pw) {
    $sec = Read-Host 'Mot de passe de la cle de signature' -AsSecureString
    $bstr = [Runtime.InteropServices.Marshal]::SecureStringToBSTR($sec)
    try { $pw = [Runtime.InteropServices.Marshal]::PtrToStringBSTR($bstr) }
    finally { [Runtime.InteropServices.Marshal]::ZeroFreeBSTR($bstr) }
}

# --- 2. Controles (avant toute modification) ---
Step 'cargo test' { cargo test --workspace }
Step 'cargo clippy' { cargo clippy --workspace --all-targets -- -D warnings }
$jsTests = @(Get-ChildItem (Join-Path $root 'ui\web\test') -Filter '*.test.mjs' | ForEach-Object { $_.FullName })
Step 'node --test' { node --test $jsTests }

# --- 3. Versions ---
if ($skipBump) {
    Write-Host "== Premiere release : version $Version deja en place."
} else {
    Write-Host "== Version $current -> $Version"
    Set-Versions -Root $root -Version $Version
    $bumped = $true
    Step 'cargo update -w' { cargo update -w }
}

# --- 4. Build ---
Step 'cargo build engine' { cargo build -p engine --release }
Copy-Item 'target\release\nullctl-engine.exe' 'ui\binaries\nullctl-engine-x86_64-pc-windows-msvc.exe' -Force
# Secrets d'environnement limites au build, restaures apres.
$saved = @{}
foreach ($n in 'TAURI_SIGNING_PRIVATE_KEY', 'TAURI_SIGNING_PRIVATE_KEY_PASSWORD') { $saved[$n] = [Environment]::GetEnvironmentVariable($n) }
Push-Location (Join-Path $root 'ui')
try {
    $env:TAURI_SIGNING_PRIVATE_KEY = $keyFile
    $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = $pw
    Step 'cargo tauri build' { cargo tauri build --config tauri.bundle.json }
}
finally {
    Pop-Location
    foreach ($n in $saved.Keys) { [Environment]::SetEnvironmentVariable($n, $saved[$n]) }
}

# --- 5. latest.json ---
$nsis = Join-Path $root 'target\release\bundle\nsis'
$name = "NULL-CONTROL_${Version}_x64-setup.exe"
$setup = Join-Path $nsis $name
$sigFile = "$setup.sig"
foreach ($f in $setup, $sigFile) { if (-not (Test-Path $f)) { Fail "Artefact manquant : $f" } }
if (-not $Notes) { $Notes = "NULL/CONTROL $Version" }
$json = New-LatestJson -Version $Version -Notes $Notes `
    -Signature ([IO.File]::ReadAllText($sigFile)) `
    -Url "https://github.com/Telooss/NULL-PAD/releases/download/v$Version/$name" `
    -Date (Get-Date).ToUniversalTime()
$latest = Join-Path $nsis 'latest.json'
[IO.File]::WriteAllText($latest, $json, (New-Object Text.UTF8Encoding $false))
Write-Host "== $latest ecrit"

# --- 6. Commit et tag ---
$allowed = 'ui/tauri.conf.json', 'ui/Cargo.toml', 'engine/Cargo.toml', 'Cargo.lock'
$changed = @(git status --porcelain | ForEach-Object { $_.Substring(3).Trim('"') })
$other = @($changed | Where-Object { $allowed -notcontains $_ })
if ($other.Count -gt 0) { Fail ("Fichiers inattendus modifies, je m'arrete : " + ($other -join ', ')) }
if ($changed.Count -gt 0) {
    git add -- $allowed
    if ($LASTEXITCODE -ne 0) { Fail 'git add a echoue.' }
    git commit -m "release: v$Version" -m 'Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>'
    if ($LASTEXITCODE -ne 0) { Fail 'git commit a echoue.' }
}
git tag -a "v$Version" -m "NULL/CONTROL $Version"
if ($LASTEXITCODE -ne 0) { Fail 'git tag a echoue.' }

# --- 7. Publication ---
$title = "NULL/CONTROL $Version"
# PS 5.1 supprime les arguments natifs vides : les notes passent par un fichier.
$notesFile = Join-Path $nsis 'notes.txt'
[IO.File]::WriteAllText($notesFile, $Notes, (New-Object Text.UTF8Encoding $false))
if ($Publish) {
    Step 'git push' { git push }
    Step 'git push tag' { git push origin "v$Version" }
    Step 'gh release create' { gh release create "v$Version" $setup $sigFile $latest --title $title --notes-file $notesFile }
    Write-Host "Release v$Version publiee."
} else {
    Write-Host "Pret. Pour publier :"
    Write-Host "  git push"
    Write-Host "  git push origin v$Version"
    Write-Host "  gh release create v$Version `"$setup`" `"$sigFile`" `"$latest`" --title `"$title`" --notes-file `"$notesFile`""
}
