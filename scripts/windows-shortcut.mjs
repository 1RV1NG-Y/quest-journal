import { execFileSync } from 'node:child_process';

// WScript.Shell can lose characters through the system ANSI code page. Use the
// Unicode Shell Link interface for both target fields and IPersistFile paths.
// Method order is the IShellLinkW vtable from the Windows SDK.
const shellLinkType = `
using System;
using System.IO;
using System.Text;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.ComTypes;

[ComImport, Guid("00021401-0000-0000-C000-000000000046")]
class ShellLinkObject {}

[ComImport, Guid("000214F9-0000-0000-C000-000000000046"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
interface IShellLinkW {
    void GetPath([Out, MarshalAs(UnmanagedType.LPWStr)] StringBuilder path, int size, IntPtr findData, uint flags);
    void GetIDList(out IntPtr idList);
    void SetIDList(IntPtr idList);
    void GetDescription([Out, MarshalAs(UnmanagedType.LPWStr)] StringBuilder description, int size);
    void SetDescription([MarshalAs(UnmanagedType.LPWStr)] string description);
    void GetWorkingDirectory([Out, MarshalAs(UnmanagedType.LPWStr)] StringBuilder directory, int size);
    void SetWorkingDirectory([MarshalAs(UnmanagedType.LPWStr)] string directory);
    void GetArguments([Out, MarshalAs(UnmanagedType.LPWStr)] StringBuilder arguments, int size);
    void SetArguments([MarshalAs(UnmanagedType.LPWStr)] string arguments);
    void GetHotkey(out short hotkey);
    void SetHotkey(short hotkey);
    void GetShowCmd(out int command);
    void SetShowCmd(int command);
    void GetIconLocation([Out, MarshalAs(UnmanagedType.LPWStr)] StringBuilder path, int size, out int index);
    void SetIconLocation([MarshalAs(UnmanagedType.LPWStr)] string path, int index);
    void SetRelativePath([MarshalAs(UnmanagedType.LPWStr)] string path, uint reserved);
    void Resolve(IntPtr window, uint flags);
    void SetPath([MarshalAs(UnmanagedType.LPWStr)] string path);
}

public static class QuestShortcut {
    public static void Create(string shortcut, string target) {
        var link = (IShellLinkW)new ShellLinkObject();
        try {
            link.SetPath(target);
            link.SetWorkingDirectory(Path.GetDirectoryName(target));
            link.SetDescription("Quest Journal");
            link.SetIconLocation(target, 0);
            ((IPersistFile)link).Save(shortcut, true);
        } finally { Marshal.FinalReleaseComObject(link); }
    }

    public static string[] Read(string shortcut) {
        var link = (IShellLinkW)new ShellLinkObject();
        try {
            ((IPersistFile)link).Load(shortcut, 0);
            var target = new StringBuilder(32768);
            var directory = new StringBuilder(32768);
            var icon = new StringBuilder(32768);
            var arguments = new StringBuilder(32768);
            int iconIndex;
            link.GetPath(target, target.Capacity, IntPtr.Zero, 4);
            link.GetWorkingDirectory(directory, directory.Capacity);
            link.GetIconLocation(icon, icon.Capacity, out iconIndex);
            link.GetArguments(arguments, arguments.Capacity);
            return new [] { target.ToString(), directory.ToString(), icon.ToString(), iconIndex.ToString(), arguments.ToString() };
        } finally { Marshal.FinalReleaseComObject(link); }
    }
}
`;

function runShortcut(command, environment) {
  if (process.platform !== 'win32') throw new Error('Shell shortcuts require Windows.');
  const script = `$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
[Console]::OutputEncoding = New-Object System.Text.UTF8Encoding($false)
Add-Type -TypeDefinition @'
${shellLinkType}
'@
${command}`;
  return execFileSync('powershell.exe', ['-NoProfile', '-NonInteractive', '-STA', '-EncodedCommand', Buffer.from(script, 'utf16le').toString('base64')], {
    encoding: 'utf8', windowsHide: true,
    // Paths are data, never PowerShell source or shell-interpolated arguments.
    env: { ...process.env, ...environment },
  });
}

export function createWindowsShortcut(shortcut, target) {
  runShortcut('[QuestShortcut]::Create($env:QUEST_SHORTCUT_PATH, $env:QUEST_SHORTCUT_TARGET)', {
    QUEST_SHORTCUT_PATH: shortcut, QUEST_SHORTCUT_TARGET: target,
  });
}

export function readWindowsShortcut(shortcut) {
  const fields = JSON.parse(runShortcut('[Console]::Write((ConvertTo-Json -Compress -InputObject ([QuestShortcut]::Read($env:QUEST_SHORTCUT_PATH))))', {
    QUEST_SHORTCUT_PATH: shortcut,
  }));
  return { target: fields[0], workingDirectory: fields[1], icon: `${fields[2]},${fields[3]}`, arguments: fields[4] };
}
