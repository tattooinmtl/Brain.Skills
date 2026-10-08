package cmd

import (
	"fmt"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strings"
)

func pathExists(path string) bool {
	if strings.Contains(path, "*") {
		matches, _ := filepath.Glob(path)
		return len(matches) > 0
	}
	_, err := os.Stat(path)
	return !os.IsNotExist(err)
}

func isJunctionOrSymlink(path string) bool {
	fi, err := os.Lstat(path)
	if err != nil {
		return false
	}
	// Since Go 1.23, Windows junctions (mount points) report ModeIrregular,
	// not ModeSymlink. Treat any reparse point we can read a target from as a link.
	if fi.Mode()&os.ModeSymlink != 0 {
		return true
	}
	if fi.Mode()&os.ModeIrregular != 0 {
		if _, err := os.Readlink(path); err == nil {
			return true
		}
	}
	return false
}

func createJunction(targetPath, linkPath string) error {
	// Clean paths to absolute
	absTarget, err := filepath.Abs(targetPath)
	if err != nil {
		absTarget = targetPath
	}
	absLink, err := filepath.Abs(linkPath)
	if err != nil {
		absLink = linkPath
	}

	// Ensure parent dir exists
	parent := filepath.Dir(absLink)
	if err := os.MkdirAll(parent, 0755); err != nil {
		return err
	}

	if runtime.GOOS == "windows" {
		// Windows: use cmd /c mklink /J (unprivileged directory junction)
		cmd := exec.Command("cmd", "/c", "mklink", "/J", absLink, absTarget)
		output, err := cmd.CombinedOutput()
		if err != nil {
			return fmt.Errorf("junction failed: %v (%s)", err, strings.TrimSpace(string(output)))
		}
		return nil
	}

	// Unix / macOS: standard symlink
	return os.Symlink(absTarget, absLink)
}

func removeJunction(linkPath string) error {
	if runtime.GOOS == "windows" {
		cmd := exec.Command("cmd", "/c", "rmdir", linkPath)
		return cmd.Run()
	}
	return os.Remove(linkPath)
}

func copyDir(src, dst string) error {
	return filepath.Walk(src, func(path string, info os.FileInfo, err error) error {
		if err != nil {
			return err
		}
		relPath, err := filepath.Rel(src, path)
		if err != nil {
			return err
		}
		targetPath := filepath.Join(dst, relPath)
		if info.IsDir() {
			return os.MkdirAll(targetPath, info.Mode())
		}
		return copyFile(path, targetPath)
	})
}

func copyFile(src, dst string) error {
	in, err := os.Open(src)
	if err != nil {
		return err
	}
	defer in.Close()

	if err := os.MkdirAll(filepath.Dir(dst), 0755); err != nil {
		return err
	}

	out, err := os.Create(dst)
	if err != nil {
		return err
	}
	defer out.Close()

	_, err = io.Copy(out, in)
	return err
}

func getHomeDir() string {
	home, err := os.UserHomeDir()
	if err == nil && home != "" {
		return home
	}
	if h := os.Getenv("USERPROFILE"); h != "" {
		return h
	}
	return os.Getenv("HOME")
}

func getRepoRoot() string {
	if r := os.Getenv("SKILLS_ROOT"); r != "" {
		return r
	}
	// Try looking upwards from current directory or executable location
	exe, err := os.Executable()
	if err == nil {
		dir := filepath.Dir(exe)
		if pathExists(filepath.Join(dir, "skills")) {
			return dir
		}
		if pathExists(filepath.Join(dir, "..", "skills")) {
			return filepath.Clean(filepath.Join(dir, ".."))
		}
	}
	cwd, err := os.Getwd()
	if err == nil {
		curr := cwd
		for i := 0; i < 5; i++ {
			if pathExists(filepath.Join(curr, "skills")) {
				return curr
			}
			parent := filepath.Dir(curr)
			if parent == curr {
				break
			}
			curr = parent
		}
	}
	return "C:\\.skills"
}

func getSystemBlocklist() map[string]bool {
	return map[string]bool{
		".git": true, ".gitconfig": true, ".gitignore": true, ".gitattributes": true,
		".ssh": true, ".gnupg": true, ".pki": true,
		".vscode": true, ".vscode-shared": true, ".eclipse": true, ".idlerc": true,
		".aws": true, ".azure": true, ".docker": true, ".oci": true, ".gcp": true,
		".cache": true, ".local": true, ".cargo": true, ".rustup": true,
		".npm": true, ".nuget": true, ".yarn": true, ".bun": true, ".node_repl_history": true,
		".m2": true, ".gradle": true, ".dotnet": true,
		".bash_history": true, ".ipython": true, ".thumbnails": true,
		".oh-my-posh": true, ".vagrant": true, ".thunderbird": true, ".mozilla": true,
		".VirtualBox": true,
	}
}
