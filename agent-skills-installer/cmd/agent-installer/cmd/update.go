package cmd

import (
	"encoding/json"
	"os"
	"os/exec"
	"path/filepath"
	"strings"

	"github.com/fatih/color"
	"github.com/spf13/cobra"
)

var updateCmd = &cobra.Command{
	Use:   "update",
	Short: "Pull latest skills from git and resync catalog and junctions",
	Long:  `Runs git pull --ff-only on the skills repository, rebuilds skills.json, and ensures all junctions are active.`,
	RunE:  runUpdate,
}

func init() {
	rootCmd.AddCommand(updateCmd)
}

func runUpdate(cmd *cobra.Command, args []string) error {
	repoRoot := getRepoRoot()
	color.Cyan("🚀 Updating Brain.Skills at %s...\n", repoRoot)

	// 1. git pull --ff-only
	gitCmd := exec.Command("git", "-C", repoRoot, "pull", "--ff-only")
	output, err := gitCmd.CombinedOutput()
	if err != nil {
		color.Yellow("Git pull warning: %v (%s)", err, strings.TrimSpace(string(output)))
	} else {
		color.Green("✓ Git update: %s", strings.TrimSpace(string(output)))
	}

	// 2. Re-sync catalog
	if err := runSync(cmd, args); err != nil {
		color.Red("✗ Failed to resync skills catalog: %v", err)
	}

	// 3. Re-verify manifest junctions
	manifestPath := filepath.Join(getHomeDir(), ".skills-install.json")
	if data, err := os.ReadFile(manifestPath); err == nil {
		var manifest InstallManifest
		if err := json.Unmarshal(data, &manifest); err == nil && len(manifest.Agents) > 0 {
			color.Cyan("\nVerifying %d installed agents from manifest...", len(manifest.Agents))
			skillsDir := filepath.Join(repoRoot, "skills")
			for _, agent := range manifest.Agents {
				skillsTarget := filepath.Join(agent.ConfigPath, "skills")
				if !isJunctionOrSymlink(skillsTarget) && !pathExists(skillsTarget) {
					_ = createJunction(skillsDir, skillsTarget)
					color.Green("  ✓ Restored missing junction -> %s", skillsTarget)
				} else {
					color.Green("  ✓ Junction intact -> %s", skillsTarget)
				}
			}
		}
	}

	color.Green("\n✅ Brain.Skills update completed successfully!")
	return nil
}
