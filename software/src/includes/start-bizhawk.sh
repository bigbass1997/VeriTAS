#!/bin/sh
cd "$(dirname "$(realpath "$0")")"
### This is commented to allow for multiple instances of bizhawk for dumping purposes
#if [ "$(ps -C "mono" -o "cmd" --no-headers | grep "EmuHawk.exe")" ]; then
#	echo "EmuHawk is already running, exiting..."
#	exit 0
#fi
libpath=""
winepath=""
if [ "$(command -v lsb_release)" ]; then
	case "$(lsb_release -i | cut -c17- | tr -d "\n" | tr A-Z a-z)" in
		"arch"|"manjarolinux"|"artix") libpath="/usr/lib";;
		"fedora") libpath="/usr/lib64"; export MONO_WINFORMS_XIM_STYLE=disabled;; # see https://bugzilla.xamarin.com/show_bug.cgi?id=28047#c9
		"debian"|"linuxmint"|"ubuntu"|"pop") libpath="/usr/lib/x86_64-linux-gnu"; export MONO_WINFORMS_XIM_STYLE=disabled;; # ditto
	esac
else
	printf "Distro does not provide LSB release info API! (You've met with a terrible fate, haven't you?)\n"
fi
if [ -z "$libpath" ]; then
	printf "%s\n" "Unknown distro, assuming system-wide libraries are in /usr/lib..."
	libpath="/usr/lib"
fi
if [ -z "$winepath" ]; then winepath="$libpath/wine"; fi
export LD_LIBRARY_PATH="$PWD/dll:$PWD:$winepath:$libpath"
export BIZHAWK_INT_SYSLIB_PATH="$libpath"
if [ "$1" = "--mono-no-redirect" ]; then
	shift
	printf "(received --mono-no-redirect, stdout was not captured)\n" >EmuHawkMono_laststdout.txt
	printf "(received --mono-no-redirect, stderr was not captured)\n" >EmuHawkMono_laststderr.txt
	mono ./EmuHawk.exe "$@"
else
	mono ./EmuHawk.exe "$@" >EmuHawkMono_laststdout.txt 2>EmuHawkMono_laststderr.txt
fi
