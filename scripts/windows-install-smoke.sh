#!/usr/bin/env bash
# Install and launch a per-user Windows setup .exe, and check what it left behind (feature 030,
# FR-018).
#
#   scripts/windows-install-smoke.sh <micold-ai-ide-<version>-<arch>-setup.exe>
#
# CI runs it on both Windows packaging legs, and the release runs it before each upload. Contract:
# specs/030-windows-installer/contracts/windows-installer.md, rows I1, I2, I4, I5 and I7. Its host and
# argument checks are driven by scripts/tests/windows-install-smoke.test.sh; the rest only runs on
# Windows.

set -euo pipefail

case "$(uname -s)" in
MINGW* | MSYS* | CYGWIN*) ;;
*)
	echo "${0##*/}: the Windows installer is smoke-tested on Windows (it installs a real setup .exe)" >&2
	exit 1
	;;
esac

if [ "$#" -ne 1 ]; then
	echo "usage: ${0##*/} <setup.exe>" >&2
	exit 2
fi
exe="$1"
if [ ! -f "$exe" ]; then
	echo "${0##*/}: no setup executable at '$exe'" >&2
	exit 2
fi

# Pinned in packaging/windows/micold-ai-ide.iss; Inno Setup keys the uninstall entry on it.
app_id='{1B19A6AC-4C91-4033-88EA-F7F283127C8A}'
# How long the daemon's pipe may take to appear after the client starts (I2).
pipe_wait_secs=20
# How long the uninstaller may take to remove the install once it is started (I5).
uninstall_wait_secs=60

root="$(cd "$(dirname "$0")/.." && pwd)"
version="$(sed -n '/^\[workspace\.package\]/,/^\[/s/^version *= *"\(.*\)"/\1/p' "$root/Cargo.toml")"
if [ -z "$version" ]; then
	echo "${0##*/}: no [workspace.package] version in $root/Cargo.toml" >&2
	exit 1
fi

# Every question to Windows goes through PowerShell: MSYS2 rewrites a `/switch`-looking argument into
# a path, and `$!` is an MSYS2 pid, not a Windows one. Output loses its CRs.
win() {
	MSYS2_ARG_CONV_EXCL='*' powershell.exe -NoProfile -NonInteractive -Command "$1" | tr -d '\r'
}

# `True` while the Windows process <pid> exists.
alive() {
	win "[bool](Get-Process -Id $1 -ErrorAction SilentlyContinue)"
}

local_app_data="$(cygpath -u "$LOCALAPPDATA")"
app_data="$(cygpath -u "$APPDATA")"
install_dir="$local_app_data/Programs/Micold AI IDE"
logs="$(mktemp -d)"
# What stands in for a user's projects and settings, which an uninstall keeps (I5, FR-007).
markers=("$app_data/micold-ai-ide/data/smoke-marker" "$local_app_data/micold-ai-ide/data/smoke-marker")
client_pid=""
daemon_pid=""

# Stops only the processes this run started, never one by image name: on a developer's PC another
# Micold AI IDE may be open.
clean_up() {
	if [ -n "$client_pid" ]; then
		MSYS2_ARG_CONV_EXCL='*' taskkill.exe /PID "$client_pid" /T /F >/dev/null 2>&1 || true
	fi
	if [ -n "$daemon_pid" ]; then
		win "Stop-Process -Id $daemon_pid -Force -ErrorAction SilentlyContinue" || true
	fi
	rm -f "${markers[@]}"
	rm -rf "$logs"
}
trap clean_up EXIT

fail() {
	echo "${0##*/}: FAIL: $*" >&2
	for log in "$logs/install.log" "$logs/repair.log" "$logs/uninstall.log" "$local_app_data/micold-ai-ide/data/micold-daemon.log"; do
		if [ -f "$log" ]; then
			echo "---- $log" >&2
			cat "$log" >&2
		fi
	done
	exit 1
}

for marker in "${markers[@]}"; do
	mkdir -p "$(dirname "$marker")"
	echo "seeded by ${0##*/}" >"$marker"
done

# 1. The silent install succeeds (I7).
echo "== install $exe"
status=0
MSYS2_ARG_CONV_EXCL='*' "$exe" /VERYSILENT /SUPPRESSMSGBOXES /NORESTART "/LOG=$(cygpath -w "$logs/install.log")" ||
	status=$?
[ "$status" -eq 0 ] || fail "the silent install exited $status, want 0 (I7)"

# 2. Per user: both exes under %LOCALAPPDATA%\Programs, and nothing needed admin (I1, FR-002, FR-004).
for bin in micold-ai-ide.exe micold-daemon.exe; do
	[ -f "$install_dir/$bin" ] || fail "$bin is not in $install_dir (I1)"
done

# 2a. Each exe carries an icon group resource, which is what Explorer, the taskbar and Installed apps
# show for it (FR-003). Without one Windows draws its generic exe icon.
cat >"$logs/group-icons.ps1" <<'PS1'
param([string]$Path)
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class MicoldResources {
    delegate bool EnumNames(IntPtr module, IntPtr type, IntPtr name, IntPtr param);
    [DllImport("kernel32.dll", SetLastError = true, CharSet = CharSet.Unicode)]
    static extern IntPtr LoadLibraryExW(string path, IntPtr file, uint flags);
    [DllImport("kernel32.dll")]
    static extern bool EnumResourceNamesW(IntPtr module, IntPtr type, EnumNames callback, IntPtr param);
    [DllImport("kernel32.dll")]
    static extern bool FreeLibrary(IntPtr module);
    // RT_GROUP_ICON resources in the image at path, or -1 if it cannot be loaded as a data file.
    public static int GroupIcons(string path) {
        IntPtr module = LoadLibraryExW(path, IntPtr.Zero, 0x22); // AS_DATAFILE | AS_IMAGE_RESOURCE
        if (module == IntPtr.Zero) return -1;
        int count = 0;
        try {
            EnumResourceNamesW(module, (IntPtr)14, (m, t, n, p) => { count++; return true; }, IntPtr.Zero);
        } finally {
            FreeLibrary(module);
        }
        return count;
    }
}
'@
[MicoldResources]::GroupIcons($Path)
PS1
for bin in micold-ai-ide.exe micold-daemon.exe; do
	icons="$(MSYS2_ARG_CONV_EXCL='*' powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass \
		-File "$(cygpath -w "$logs/group-icons.ps1")" "$(cygpath -w "$install_dir/$bin")" | tr -d '\r')"
	[ "$icons" -ge 1 ] 2>/dev/null || fail "$bin has no icon resource (found '$icons'), so Windows shows a generic icon (FR-003)"
done
echo "both exes carry an icon"

# 3. The Start menu entry (I1).
shortcut="$app_data/Microsoft/Windows/Start Menu/Programs/Micold AI IDE.lnk"
[ -f "$shortcut" ] || fail "no Start menu shortcut at $shortcut (I1)"

# 4. The Installed apps entry carries the workspace version (I1, FR-013).
key="HKCU:\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\${app_id}_is1"
displayed="$(win "(Get-ItemProperty -LiteralPath '$key' -ErrorAction Stop).DisplayVersion")" ||
	fail "no uninstall key $key (I1)"
[ "$displayed" = "$version" ] ||
	fail "the uninstall key's DisplayVersion is '$displayed', want '$version' (FR-013)"
echo "installed $version to $install_dir"

# 5. Launch the installed client detached, through ShellExecute as the Start menu does. Redirecting
# its output instead would hand a console-subsystem client PowerShell's own console, and step 7
# could never see the window a user would.
client_pid="$(win "(Start-Process -FilePath '$(cygpath -w "$install_dir/micold-ai-ide.exe")' -PassThru).Id")"
[ -n "$client_pid" ] || fail "the installed client did not start"
echo "== launched the client, pid $client_pid"

# 6. The client found its sibling daemon and spawned it: this user's pipe appears (I2, FR-020).
sid="$(win '[System.Security.Principal.WindowsIdentity]::GetCurrent().User.Value')"
pipe="\\\\.\\pipe\\Micold.Daemon.$sid"
started=$SECONDS
until [ "$(win "Test-Path -LiteralPath '$pipe'")" = True ]; do
	[ "$(alive "$client_pid")" = True ] || fail "the client (pid $client_pid) exited before $pipe appeared"
	[ $((SECONDS - started)) -lt "$pipe_wait_secs" ] || fail "$pipe did not appear within ${pipe_wait_secs}s (I2)"
	sleep 1
done
echo "$pipe appeared after $((SECONDS - started))s"

# The daemon: the pid it records, else the micold-daemon.exe the client spawned.
pid_record="$local_app_data/micold-ai-ide/run/micold-daemon.pid"
if [ -f "$pid_record" ]; then
	daemon_pid="$(tr -dc '0-9' <"$pid_record")"
fi
if [ -z "$daemon_pid" ]; then
	daemon_pid="$(win "(Get-CimInstance Win32_Process |
Where-Object { \$_.ParentProcessId -eq $client_pid -and \$_.Name -eq 'micold-daemon.exe' } |
Select-Object -First 1).ProcessId")"
fi
[ -n "$daemon_pid" ] || fail "$pipe is up, but no micold-daemon.exe process was found behind it"

# 7. No console for the client or its daemon (FR-005, SC-005). Checked while both are still running:
# a console window's conhost.exe is gone with the process it served.
consoles="$(win "Get-CimInstance Win32_Process |
Where-Object { \$_.Name -eq 'conhost.exe' -and @($client_pid, $daemon_pid) -contains \$_.ParentProcessId } |
ForEach-Object { 'conhost.exe {0} (parent {1})' -f \$_.ProcessId, \$_.ParentProcessId }")"
[ -z "$consoles" ] ||
	fail "a console was opened for the client (pid $client_pid) or the daemon (pid $daemon_pid): $consoles (FR-005)"
echo "no console for the client (pid $client_pid) or the daemon (pid $daemon_pid)"

# Both still running once checked. There is no drawing-surface exemption as 028's macOS step has
# (its research R12): a ShellExecute launch leaves no stderr to tell a surface error from any other
# exit, so every exit fails.
for pid in "$client_pid" "$daemon_pid"; do
	[ "$(alive "$pid")" = True ] || fail "pid $pid exited after $pipe appeared"
done

# 8. Repair over a live install (I4, FR-008). A user closes the window and runs the installer again,
# while the daemon outlives the window by design. Stopping the client without /T leaves the daemon,
# which the client spawned, running.
MSYS2_ARG_CONV_EXCL='*' taskkill.exe /PID "$client_pid" /F >/dev/null 2>&1 ||
	fail "could not stop the client (pid $client_pid)"
client_pid=""
[ "$(alive "$daemon_pid")" = True ] || fail "the daemon (pid $daemon_pid) exited with its client, before the repair"
echo "== repair $exe with the daemon (pid $daemon_pid) running"
status=0
MSYS2_ARG_CONV_EXCL='*' "$exe" /VERYSILENT /SUPPRESSMSGBOXES /NORESTART "/LOG=$(cygpath -w "$logs/repair.log")" ||
	status=$?
[ "$status" -eq 0 ] || fail "the repair with the daemon running exited $status, want 0 (I4)"
entries="$(win "@(Get-ChildItem -LiteralPath 'HKCU:\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall' |
Where-Object { \$_.GetValue('DisplayName') -like 'Micold AI IDE*' }).Count")"
[ "$entries" = 1 ] || fail "$entries Installed apps entries for Micold AI IDE after the repair, want 1 (I4)"
echo "repaired, one Installed apps entry"

# The repair stopped the old daemon rather than replacing its files underneath it (I4, FR-009). Its
# pid counts as gone once no micold-daemon.exe holds it, so a pid Windows reuses does not fail this.
survivor="$(win "[bool](Get-Process -Id $daemon_pid -ErrorAction SilentlyContinue |
Where-Object { \$_.ProcessName -eq 'micold-daemon' })")"
[ "$survivor" = False ] || fail "the old daemon (pid $daemon_pid) is still running after the repair (I4)"
echo "the old daemon (pid $daemon_pid) is gone"
daemon_pid=""

# unins000.exe hands over to a copy of itself in %TEMP% and exits before the removal is done; wait for
# that copy. Inno Setup 6.7 names it _unins.tmp (older releases _iu*.tmp), and a wait that matches no
# process returns at once, so the checks after it race the removal.
wait_for_uninstaller() {
	local started=$SECONDS
	until [ "$(win "@(Get-CimInstance Win32_Process | Where-Object { \$_.Name -like '_unins*' -or \$_.Name -like '_iu*' -or \$_.Name -eq 'unins000.exe' }).Count")" = 0 ]; do
		[ $((SECONDS - started)) -lt "$uninstall_wait_secs" ] || fail "the uninstaller still runs after ${uninstall_wait_secs}s"
		sleep 1
	done
}

# 9. Uninstall as Installed apps would run it, with the daemon running (I5, I7, FR-006). Relaunch the
# client so it spawns a daemon, then close the window as in step 8. A silent uninstall with the
# window still open cancels at the app-mutex prompt, as FR-009 requires with no one to confirm.
client_pid="$(win "(Start-Process -FilePath '$(cygpath -w "$install_dir/micold-ai-ide.exe")' -PassThru).Id")"
[ -n "$client_pid" ] || fail "the repaired client did not start"
started=$SECONDS
until [ "$(win "Test-Path -LiteralPath '$pipe'")" = True ]; do
	[ "$(alive "$client_pid")" = True ] || fail "the repaired client (pid $client_pid) exited before $pipe appeared"
	[ $((SECONDS - started)) -lt "$pipe_wait_secs" ] || fail "$pipe did not come back within ${pipe_wait_secs}s"
	sleep 1
done
daemon_pid="$(win "(Get-CimInstance Win32_Process |
Where-Object { \$_.ParentProcessId -eq $client_pid -and \$_.Name -eq 'micold-daemon.exe' } |
Select-Object -First 1).ProcessId")"
MSYS2_ARG_CONV_EXCL='*' taskkill.exe /PID "$client_pid" /F >/dev/null 2>&1 ||
	fail "could not stop the repaired client (pid $client_pid)"
client_pid=""
echo "== uninstall with the daemon (pid ${daemon_pid:-unknown}) running"
status=0
MSYS2_ARG_CONV_EXCL='*' "$install_dir/unins000.exe" /VERYSILENT /SUPPRESSMSGBOXES /NORESTART \
	"/LOG=$(cygpath -w "$logs/uninstall.log")" || status=$?
[ "$status" -eq 0 ] || fail "the silent uninstall exited $status, want 0 (I7)"
wait_for_uninstaller
[ ! -e "$install_dir" ] || fail "$install_dir is still there after the uninstall (I5): $(ls -A "$install_dir" | tr '\n' ' ')"
[ ! -e "$shortcut" ] || fail "the Start menu shortcut $shortcut is still there after the uninstall (I5)"
[ "$(win "Test-Path -LiteralPath '$key'")" = False ] || fail "the uninstall key $key is still there after the uninstall (I5)"
[ ! -e "$local_app_data/micold-ai-ide/run" ] ||
	fail "$local_app_data/micold-ai-ide/run is still there after the uninstall (I5)"
echo "uninstalled: no install dir, shortcut, uninstall key or run dir"
for marker in "${markers[@]}"; do
	[ -f "$marker" ] || fail "the uninstall removed user data: $marker is gone (I5, FR-007)"
done
echo "both data markers survived the uninstall"

# clean_up stops what is still running on exit.
echo "== smoke passed"
