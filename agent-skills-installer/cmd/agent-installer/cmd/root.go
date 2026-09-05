package cmd

import (
	"fmt"

	"github.com/fatih/color"
	"github.com/spf13/cobra"
)

var rootCmd = &cobra.Command{
	Use:   "skills",
	Short: "Brain.Skills CLI Launcher — Universal AI-Agent Skills & Brain Manager",
	Long: `Brain.Skills — High-performance native Go launcher and Rust memory system.
Manages AI agent skills, directory junctions, and the native Rust brain daemon.

Commands:
  scan     Discover AI agents, custom harnesses, and dot-directories on your system
  install  Configure agents with zero-copy directory junctions and commands
  sync     Scan skills/ catalog and generate skills.json and INDEX.md natively
  update   Pull latest skills from git and resync catalog and junctions
  brain    Control the native Rust background tray daemon and memory API`,
	Run: func(cmd *cobra.Command, args []string) {
		color.Cyan("\n🧠 Brain.Skills CLI (Native Go & Rust Architecture)\n")
		fmt.Println("Available commands:")
		fmt.Println("  skills scan     - Scan system for AI agents, harnesses, and dot-directories")
		fmt.Println("  skills install  - Install skills into target agents via directory junctions")
		fmt.Println("  skills sync     - Re-generate skills.json and INDEX.md from skills/")
		fmt.Println("  skills update   - Git pull latest skills and resync all agents")
		fmt.Println("  skills brain    - Manage native Rust tray daemon and memory API")
		fmt.Println("\nRun 'skills <command> --help' for details on any command.")
	},
}

func Execute() error {
	return rootCmd.Execute()
}

func init() {
	rootCmd.AddCommand(scanCmd)
	rootCmd.AddCommand(installCmd)
	// syncCmd, updateCmd, brainCmd register themselves in their init()
}