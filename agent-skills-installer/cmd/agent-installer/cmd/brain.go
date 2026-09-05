package cmd

import (
	"fmt"
	"io"
	"net/http"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"time"

	"github.com/fatih/color"
	"github.com/spf13/cobra"
)

var brainCmd = &cobra.Command{
	Use:   "brain [start|verify|status|install-startup|uninstall-startup]",
	Short: "Manage the native Rust brain-system daemon",
	Long:  `Control the background brain-system.exe (system tray, memory vault HTTP API, verification engine).`,
	RunE:  runBrain,
}

func init() {
	rootCmd.AddCommand(brainCmd)
}

func runBrain(cmd *cobra.Command, args []string) error {
	repoRoot := getRepoRoot()
	brainExe := filepath.Join(repoRoot, "bin", "brain-system.exe")

	if !pathExists(brainExe) {
		return fmt.Errorf("brain-system executable not found at: %s", brainExe)
	}

	sub := "status"
	if len(args) > 0 {
		sub = strings.ToLower(args[0])
	}

	switch sub {
	case "start":
		color.Cyan("🧠 Launching native Rust brain-system tray daemon...")
		execCmd := exec.Command(brainExe)
		if err := execCmd.Start(); err != nil {
			return fmt.Errorf("failed to start brain-system: %v", err)
		}
		color.Green("✓ brain-system running in background (PID: %d)", execCmd.Process.Pid)
		color.Cyan("  Web UI: http://127.0.0.1:6789")

	case "verify":
		color.Cyan("⚖️  Running memory vault verification sweep...")
		execCmd := exec.Command(brainExe, "--verify")
		execCmd.Stdout = os.Stdout
		execCmd.Stderr = os.Stderr
		return execCmd.Run()

	case "install-startup":
		color.Cyan("Registering brain-system with Windows startup...")
		execCmd := exec.Command(brainExe, "--install-startup")
		execCmd.Stdout = os.Stdout
		execCmd.Stderr = os.Stderr
		return execCmd.Run()

	case "uninstall-startup":
		color.Cyan("Removing brain-system from Windows startup...")
		execCmd := exec.Command(brainExe, "--uninstall-startup")
		execCmd.Stdout = os.Stdout
		execCmd.Stderr = os.Stderr
		return execCmd.Run()

	case "status":
		color.Cyan("Checking brain-system status...")
		client := http.Client{Timeout: 1 * time.Second}
		resp, err := client.Get("http://127.0.0.1:6789/api/status")
		if err != nil {
			color.Yellow("• brain-system is not currently running on http://127.0.0.1:6789")
			fmt.Println("  Start it with: skills brain start")
		} else {
			defer resp.Body.Close()
			body, _ := io.ReadAll(resp.Body)
			color.Green("✓ brain-system is active (status %d): %s", resp.StatusCode, strings.TrimSpace(string(body)))
		}

	default:
		color.Yellow("Unknown brain command: %s. Available: start, verify, status, install-startup, uninstall-startup", sub)
	}

	return nil
}
