package cmd

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"time"

	"github.com/fatih/color"
	"github.com/spf13/cobra"
	"gopkg.in/yaml.v3"
)

var (
	skillsSourcePath string
	selectedAgents   []string
	dryRun           bool
	forceInstall     bool
	autoInstall      bool
	allInstall       bool
	useCopy          bool
	repointLinks     bool
)

var installCmd = &cobra.Command{
	Use:   "install",
	Short: "Install skills library and hooks to detected or selected agents",
	Long: `Configures AI agents by creating directory junctions to the skills repository,
installing slash commands, and wiring hooks. Uses native Windows directory junctions
(no admin required) or symlinks on Unix.`,
	RunE: runInstall,
}

func init() {
	installCmd.Flags().StringVarP(&skillsSourcePath, "skills-source", "s", "", "Path to skills directory (defaults to repo skills/)")
	installCmd.Flags().StringSliceVarP(&selectedAgents, "agents", "a", []string{}, "Target agent names (comma-separated)")
	installCmd.Flags().BoolVarP(&dryRun, "dry-run", "d", false, "Preview changes without modifying system")
	installCmd.Flags().BoolVarP(&forceInstall, "force", "f", false, "Overwrite existing targets")
	installCmd.Flags().BoolVar(&autoInstall, "auto", false, "Non-interactive: install to detected default agents only")
	installCmd.Flags().BoolVar(&allInstall, "all", false, "Non-interactive: install to ALL detected agents")
	installCmd.Flags().BoolVar(&useCopy, "copy", false, "Copy files instead of creating directory junctions")
	installCmd.Flags().BoolVar(&repointLinks, "repoint", false, "Re-point existing skills links to this source (never touches real folders)")
}

func runInstall(cmd *cobra.Command, args []string) error {
	repoRoot := getRepoRoot()
	if skillsSourcePath == "" {
		skillsSourcePath = filepath.Join(repoRoot, "skills")
	}

	color.Cyan("📦 Brain.Skills Installer\n")
	fmt.Printf("Repository root: %s\n", color.WhiteString(repoRoot))
	fmt.Printf("Skills source:   %s\n\n", color.WhiteString(skillsSourcePath))

	if !pathExists(skillsSourcePath) {
		return fmt.Errorf("skills source directory not found: %s", skillsSourcePath)
	}

	var targets []AgentInfo

	if allInstall {
		targets = discoverAgents()
	} else if autoInstall {
		for _, a := range discoverAgents() {
			if a.Tier == "default" && a.Exists {
				targets = append(targets, a)
			}
		}
	} else if len(selectedAgents) > 0 {
		all := discoverAgents()
		targets = filterAgents(all, selectedAgents)
	} else {
		// Try loading from saved selection or interactive
		targets = loadSelection()
		if len(targets) == 0 {
			all := discoverAgents()
			targets = interactiveSelect(all)
		}
	}

	if len(targets) == 0 {
		color.Yellow("No target agents selected. Run 'scan' or pass '--auto' / '--all' / '--agents'.")
		return nil
	}

	color.Green("Configuring %d target agent(s):\n", len(targets))
	for _, target := range targets {
		installToAgentTarget(repoRoot, skillsSourcePath, target)
	}

	// Save manifest
	if !dryRun {
		saveManifest(repoRoot, targets)
	}

	color.Green("\n✅ All done! Skills and configurations are wired.")
	return nil
}

func loadSelection() []AgentInfo {
	data, err := os.ReadFile(".agent-selection.yaml")
	if err != nil {
		return []AgentInfo{}
	}
	var agents []AgentInfo
	_ = yaml.Unmarshal(data, &agents)
	return agents
}

func filterAgents(agents []AgentInfo, names []string) []AgentInfo {
	nameSet := make(map[string]bool)
	for _, n := range names {
		nameSet[strings.ToLower(strings.TrimSpace(n))] = true
	}
	var filtered []AgentInfo
	for _, a := range agents {
		if nameSet[strings.ToLower(a.Name)] || nameSet[strings.ToLower(strings.TrimPrefix(filepath.Base(a.ConfigPath), "."))] {
			filtered = append(filtered, a)
		}
	}
	return filtered
}

func installToAgentTarget(repoRoot, skillsSource string, agent AgentInfo) {
	fmt.Printf("\n▶ %s (%s)\n", color.WhiteString(agent.DisplayName), color.CyanString(agent.ConfigPath))

	targetSkillsPath := filepath.Join(agent.ConfigPath, "skills")

	if dryRun {
		color.Yellow("  [DRY RUN] Would link skills -> %s", targetSkillsPath)
		return
	}

	// Ensure agent config root exists
	if err := os.MkdirAll(agent.ConfigPath, 0755); err != nil {
		color.Red("  ✗ Failed to create config path: %v", err)
		return
	}

	// Handle skills junction/link
	if isJunctionOrSymlink(targetSkillsPath) {
		current, _ := os.Readlink(targetSkillsPath)
		current = strings.TrimPrefix(current, `\??\`)
		wanted, _ := filepath.Abs(skillsSource)
		if current != "" && strings.EqualFold(filepath.Clean(current), filepath.Clean(wanted)) {
			color.Green("  ✓ Skills link already points here: %s", targetSkillsPath)
		} else if forceInstall || repointLinks {
			// os.Remove on a junction removes only the link, never the target's files.
			if err := os.Remove(targetSkillsPath); err != nil {
				color.Red("  ✗ Could not replace old link %s: %v", targetSkillsPath, err)
			} else if err := createSkillsBinding(skillsSource, targetSkillsPath); err != nil {
				color.Red("  ✗ Failed to link skills: %v", err)
			} else {
				color.Green("  ✓ Re-pointed skills link %s -> %s (was %s)", targetSkillsPath, skillsSource, current)
			}
		} else {
			color.Yellow("  • Skills link already exists (points to %s; use --repoint to re-point): %s", current, targetSkillsPath)
		}
	} else if pathExists(targetSkillsPath) {
		if forceInstall {
			// Never delete someone's own skills: move the folder aside.
			backup := fmt.Sprintf("%s.backup-%s", targetSkillsPath, time.Now().Format("20060102-150405"))
			if err := os.Rename(targetSkillsPath, backup); err != nil {
				color.Red("  ✗ Could not move existing skills folder aside: %v", err)
				return
			}
			color.Yellow("  • Moved existing skills folder to %s", backup)
			if err := createSkillsBinding(skillsSource, targetSkillsPath); err != nil {
				color.Red("  ✗ Failed to link skills: %v", err)
			} else {
				color.Green("  ✓ Junctioned skills repository -> %s", targetSkillsPath)
			}
		} else {
			color.Yellow("  ⏭  Kept this agent's own skills folder (use --force to move it aside and link the library): %s", targetSkillsPath)
		}
	} else {
		if err := createSkillsBinding(skillsSource, targetSkillsPath); err != nil {
			color.Red("  ✗ Failed to link skills: %v", err)
		} else {
			color.Green("  ✓ Junctioned skills repository -> %s", targetSkillsPath)
		}
	}

	// Install slash commands if Claude Code
	if strings.Contains(strings.ToLower(agent.ConfigPath), ".claude") {
		installClaudeCommands(repoRoot, agent.ConfigPath)
	}
}

func createSkillsBinding(src, dst string) error {
	if useCopy {
		return copyDir(src, dst)
	}
	return createJunction(src, dst)
}

func installClaudeCommands(repoRoot, agentConfigPath string) {
	commandsSrc := filepath.Join(repoRoot, "commands")
	commandsDst := filepath.Join(agentConfigPath, "commands")

	if !pathExists(commandsSrc) {
		return
	}

	entries, err := os.ReadDir(commandsSrc)
	if err != nil {
		return
	}

	_ = os.MkdirAll(commandsDst, 0755)
	for _, entry := range entries {
		if strings.HasSuffix(entry.Name(), ".md") {
			srcFile := filepath.Join(commandsSrc, entry.Name())
			dstFile := filepath.Join(commandsDst, entry.Name())

			content, err := os.ReadFile(srcFile)
			if err == nil {
				// Replace <REPO_ROOT> placeholder with normalized forward slash path
				cleanRoot := filepath.ToSlash(repoRoot)
				rendered := strings.ReplaceAll(string(content), "<REPO_ROOT>", cleanRoot)
				_ = os.WriteFile(dstFile, []byte(rendered), 0644)
				color.Green("  ✓ Installed slash command: /%s", strings.TrimSuffix(entry.Name(), ".md"))
			}
		}
	}
}

func saveManifest(repoRoot string, agents []AgentInfo) {
	manifest := InstallManifest{
		Version:     "0.3.0",
		RepoRoot:    filepath.ToSlash(repoRoot),
		InstalledAt: time.Now().Format(time.RFC3339),
		Agents:      agents,
	}

	manifestPath := filepath.Join(getHomeDir(), ".skills-install.json")
	data, err := json.MarshalIndent(manifest, "", "  ")
	if err == nil {
		_ = os.WriteFile(manifestPath, data, 0644)
		color.Cyan("\nSaved install record to: %s", manifestPath)
	}
}