!include "MUI2.nsh"

Name "BizClaw"
OutFile "BizClaw-Setup.exe"
InstallDir "$PROGRAMFILES64\BizClaw"

Section "Install"
    SetOutPath $INSTDIR
    File "bizclaw.exe"
    File "bizclaw-platform.exe"
    
    WriteRegStr HKLM "Software\BizClaw" "InstallPath" "$INSTDIR"
    CreateDirectory "$SMPROGRAMS\BizClaw"
    CreateShortcut "$SMPROGRAMS\BizClaw\BizClaw.lnk" "$INSTDIR\bizclaw.exe"
    CreateShortcut "$DESKTOP\BizClaw.lnk" "$INSTDIR\bizclaw.exe"
SectionEnd
