; Hooks NSIS de NULL/CONTROL (inclus par le template tauri-bundler 2.10).
; La mise a jour depuis l'appli n'en a pas besoin : l'appli arrete le moteur et pose le
; marqueur engine-restart elle-meme, et l'ancien desinstalleur n'est pas lance.

Var EngineWasRunning

; $EngineWasRunning = 1 si le mutex d'instance unique du moteur existe (engine/src/main.rs).
!macro NULLCTL_ENGINE_RUNNING
  StrCpy $EngineWasRunning 0
  System::Call 'kernel32::OpenMutexW(i 0x00100000, i 0, w "Local\nullctl-engine") p .r1'
  ${If} $1 P<> 0
    System::Call 'kernel32::CloseHandle(p r1)'
    StrCpy $EngineWasRunning 1
  ${EndIf}
!macroend

; Arrete le moteur installe : --quit (attend jusqu'a 5 s), taskkill en dernier recours,
; puis attend la fin du mutex (5 s max) pour que l'exe soit libere avant File/Delete.
!macro NULLCTL_STOP_ENGINE
  ClearErrors
  ExecWait '"$INSTDIR\nullctl-engine.exe" --quit' $0
  ${If} ${Errors}
  ${OrIf} $0 <> 0
    nsExec::Exec '"$SYSDIR\taskkill.exe" /F /IM nullctl-engine.exe'
    Pop $0
  ${EndIf}
  StrCpy $2 0
  ${Do}
    System::Call 'kernel32::OpenMutexW(i 0x00100000, i 0, w "Local\nullctl-engine") p .r1'
    ${If} $1 P= 0
      ${Break}
    ${EndIf}
    System::Call 'kernel32::CloseHandle(p r1)'
    Sleep 100
    IntOp $2 $2 + 1
  ${LoopUntil} $2 >= 50
!macroend

!macro NSIS_HOOK_PREINSTALL
  StrCpy $EngineWasRunning 0
  ${If} ${FileExists} "$INSTDIR\nullctl-engine.exe"
    !insertmacro NULLCTL_ENGINE_RUNNING
    ${If} $EngineWasRunning = 1
      !insertmacro NULLCTL_STOP_ENGINE
    ${EndIf}
  ${EndIf}
  ; Ne pas toucher a engine-restart : la mise a jour depuis l'appli en a besoin.
!macroend

!macro NSIS_HOOK_POSTINSTALL
  ${If} $EngineWasRunning = 1
    nsis_tauri_utils::RunAsUser "$INSTDIR\nullctl-engine.exe" ""
  ${EndIf}
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  StrCpy $EngineWasRunning 0
  ${If} ${FileExists} "$INSTDIR\nullctl-engine.exe"
    !insertmacro NULLCTL_ENGINE_RUNNING
    !insertmacro NULLCTL_STOP_ENGINE
  ${EndIf}
!macroend

; Ici, apres la suppression des fichiers : annuler l'invite « appli en cours » du template
; laisse le demarrage automatique intact.
; Desinstallation normale : le desinstalleur tourne depuis une copie dans %TEMP%.
; Reinstallation (mise a jour manuelle) : l'ancien desinstalleur tourne en place (_?=),
; sans /UPDATE ; on garde alors le demarrage automatique, le marqueur et la config.
!macro NSIS_HOOK_POSTUNINSTALL
  ${If} $UpdateMode <> 1 ; defense : le template ne lance pas l'ancien desinstalleur en /UPDATE
  ${AndIf} $APPDATA != ""
    ${If} $EXEDIR != $INSTDIR
      DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "nullctl"
      Delete "$APPDATA\nullctl\engine-restart"
      ; Case « Supprimer les donnees de l'application », decochee par defaut. Le template ne
      ; supprime que les dossiers nommes d'apres l'identifiant ; la config est dans %APPDATA%\nullctl.
      ${If} $DeleteAppDataCheckboxState = 1
        RMDir /r "$APPDATA\nullctl"
      ${EndIf}
    ${ElseIf} $EngineWasRunning = 1
      ; L'appli relancee apres l'installation consomme ce marqueur et relance le moteur.
      CreateDirectory "$APPDATA\nullctl"
      FileOpen $0 "$APPDATA\nullctl\engine-restart" w
      FileClose $0
    ${EndIf}
  ${EndIf}
!macroend
