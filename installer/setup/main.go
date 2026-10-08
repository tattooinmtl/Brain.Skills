// Brain.Skills setup bootstrappers.
//
// One source, two products (chosen at build time with -ldflags "-X main.product=..."):
//
//	core -> BrainSkills-Setup.exe            runs install.ps1
//	hub  -> Skills-Installer-Hub-Setup.exe   runs install-skills.ps1
//
// The setup always installs the newest version: it downloads the current
// script from the repository's main branch and runs it with Windows
// PowerShell. If GitHub can't be reached it falls back to the copy embedded
// at build time (which will itself report that it is offline). Everything
// installs per user; no administrator rights are needed.
package main

import (
	"bufio"
	_ "embed"
	"fmt"
	"io"
	"net/http"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"syscall"
	"time"
	"unsafe"
)

//go:embed scripts/install.ps1
var coreScript []byte

//go:embed scripts/install-skills.ps1
var hubScript []byte

var (
	product = "core"
	version = "dev"
)

const repo = "tattooinmtl/Brain.Skills"

func setTitle(t string) {
	p, err := syscall.UTF16PtrFromString(t)
	if err != nil {
		return
	}
	proc := syscall.NewLazyDLL("kernel32.dll").NewProc("SetConsoleTitleW")
	_, _, _ = proc.Call(uintptr(unsafe.Pointer(p)))
}

func fetch(name string) ([]byte, error) {
	client := &http.Client{Timeout: 30 * time.Second}
	url := fmt.Sprintf("https://raw.githubusercontent.com/%s/main/%s?t=%d", repo, name, time.Now().Unix())
	req, _ := http.NewRequest("GET", url, nil)
	req.Header.Set("User-Agent", "BrainSkills-Setup/"+version)
	resp, err := client.Do(req)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()
	if resp.StatusCode != 200 {
		return nil, fmt.Errorf("HTTP %d", resp.StatusCode)
	}
	b, err := io.ReadAll(io.LimitReader(resp.Body, 4<<20))
	if err != nil {
		return nil, err
	}
	if !strings.Contains(string(b), "& {") {
		return nil, fmt.Errorf("unexpected content")
	}
	return b, nil
}

func pause() {
	fmt.Print("\n  Press Enter to close this window...")
	_, _ = bufio.NewReader(os.Stdin).ReadString('\n')
}

func main() {
	title, script, embedded := "Brain.Skills Setup", "install.ps1", coreScript
	if product == "hub" {
		title, script, embedded = "Skills Installer Hub", "install-skills.ps1", hubScript
	}
	setTitle(title)
	fmt.Printf("\n  %s %s\n  github.com/%s\n", title, version, repo)

	body, err := fetch(script)
	if err != nil {
		fmt.Printf("\n  Could not download the latest installer (%v); using the built-in copy.\n", err)
		body = embedded
	}
	tmp := filepath.Join(os.TempDir(), fmt.Sprintf("brainskills-%s-%d.ps1", product, os.Getpid()))
	if err := os.WriteFile(tmp, body, 0o600); err != nil {
		fmt.Printf("\n  Could not write %s: %v\n", tmp, err)
		pause()
		os.Exit(1)
	}
	defer os.Remove(tmp)

	args := append([]string{"-NoProfile", "-ExecutionPolicy", "Bypass", "-File", tmp}, os.Args[1:]...)
	cmd := exec.Command("powershell.exe", args...)
	cmd.Stdin, cmd.Stdout, cmd.Stderr = os.Stdin, os.Stdout, os.Stderr
	runErr := cmd.Run()
	if runErr != nil {
		fmt.Printf("\n  The installer stopped with an error: %v\n", runErr)
	}
	pause()
	if runErr != nil {
		os.Exit(1)
	}
}
