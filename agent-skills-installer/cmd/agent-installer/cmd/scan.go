package cmd

import (
	"fmt"
	"os"
	"path/filepath"
	"runtime"
	"strings"

	"github.com/fatih/color"
	"github.com/manifoldco/promptui"
	"github.com/spf13/cobra"
	"gopkg.in/yaml.v3"
)

var (
	scanAllDrives   bool
	customPaths     []string
	outputFormat    string
	interactiveMode bool
)

var scanCmd = &cobra.Command{
	Use:   "scan",
	Short: "Scan system for coding agents, harnesses, and AI assistants",
	Long: `Scan user home directory (all .* folders) and drives for known coding agents,
custom harnesses, and AI assistants.`,
	RunE: runScan,
}

func init() {
	scanCmd.Flags().BoolVarP(&scanAllDrives, "all-drives", "a", false, "Scan all available drives (C:, D:, etc.)")
	scanCmd.Flags().StringSliceVarP(&customPaths, "custom", "c", []string{}, "Additional custom paths to scan")
	scanCmd.Flags().StringVarP(&outputFormat, "output", "o", "table", "Output format: table, json, yaml")
	scanCmd.Flags().BoolVarP(&interactiveMode, "interactive", "i", true, "Interactive selection")
}

func runScan(cmd *cobra.Command, args []string) error {
	color.Cyan("🔍 Scanning for coding agents and harnesses in home directory...\n")

	agents := discoverAgents()

	if len(agents) == 0 {
		color.Yellow("No agents or harnesses found in home directory.")
		return nil
	}

	displayResults(agents)

	if interactiveMode {
		selected := interactiveSelect(agents)
		if len(selected) > 0 {
			color.Green("\n✅ Selected %d agent(s):", len(selected))
			for _, a := range selected {
				fmt.Printf("  - %s (%s)\n", a.DisplayName, a.ConfigPath)
			}
			saveSelection(selected)
		}
	}

	return nil
}

func discoverAgents() []AgentInfo {
	var agents []AgentInfo
	homeDir := getHomeDir()
	blocklist := getSystemBlocklist()

	// 1. Registry of known curated agents
	knownRegistry := map[string]struct {
		name string
		tier string
		hook string
	}{
		".claude":        {name: "Claude Code CLI", tier: "default", hook: "claude"},
		".codex":         {name: "OpenAI Codex CLI", tier: "default"},
		".kimi":          {name: "Kimi Code", tier: "default"},
		".kimi-code":     {name: "Kimi Code Alternative", tier: "default"},
		".minimax":       {name: "MiniMax", tier: "default"},
		".minimax-agent": {name: "MiniMax Agent", tier: "default"},
		".mmx":           {name: "MMX (MiniMax CLI)", tier: "default"},
		".agents":        {name: "Generic Agents", tier: "default"},
		".pi":            {name: "Pi Dev Agent", tier: "default"},
		".hermes":        {name: "Hermes Harness", tier: "default"},
		".omni":          {name: "Omni Agent", tier: "default"},
		".omni-harness":  {name: "Omni Harness", tier: "default"},
		".nimagent":      {name: "NimAgent", tier: "default"},
		".NimAgent":      {name: "NimAgent (Legacy)", tier: "default"},
		".nim2":          {name: "NimAgent2", tier: "default"},
		".rustyharness":  {name: "Rusty Harness", tier: "default"},
		".gemini":        {name: "Gemini CLI", tier: "optional"},
		".grok":          {name: "Grok CLI", tier: "optional"},
		".cursor":        {name: "Cursor IDE", tier: "optional"},
		".windsurf":      {name: "Windsurf Editor", tier: "optional"},
		".aider":         {name: "Aider CLI", tier: "optional"},
		".continue":      {name: "Continue.dev", tier: "optional"},
		".dariusai":      {name: "DariusAI", tier: "optional"},
		".mavis":         {name: "Mavis", tier: "optional"},
		".agnes":         {name: "Agnes", tier: "optional"},
		".ultima":        {name: "Ultima", tier: "optional"},
		".agent-browser": {name: "Agent Browser", tier: "optional"},
		".qodo":          {name: "Qodo", tier: "optional"},
		".codegeex":      {name: "CodeGeeX", tier: "optional"},
		".codefix":       {name: "CodeFix", tier: "optional"},
		".gumiho":        {name: "Gumiho", tier: "optional"},
		".mgntx":         {name: "MGNTX", tier: "optional"},
		".ornithAI":      {name: "OrnithAI", tier: "optional"},
		".caveman":       {name: "Caveman", tier: "optional"},
		".tooluniverse":  {name: "Tool Universe", tier: "optional"},
		".neo":           {name: "Neo", tier: "optional"},
	}

	seenPaths := make(map[string]bool)

	// Read all entries in user home directory to catch all .* folders
	entries, err := os.ReadDir(homeDir)
	if err == nil {
		for _, entry := range entries {
			if !entry.IsDir() {
				continue
			}
			folderName := entry.Name()
			if !strings.HasPrefix(folderName, ".") {
				continue
			}
			if blocklist[folderName] {
				continue
			}

			fullPath := filepath.Join(homeDir, folderName)
			seenPaths[strings.ToLower(fullPath)] = true

			cleanKey := strings.ToLower(folderName)
			if known, exists := knownRegistry[cleanKey]; exists {
				agents = append(agents, AgentInfo{
					Name:        strings.TrimPrefix(folderName, "."),
					DisplayName: known.name,
					ConfigPath:  fullPath,
					ConfigType:  "folder",
					Exists:      true,
					HooksPath:   filepath.Join(fullPath, "hooks"),
					SkillsPath:  filepath.Join(fullPath, "skills"),
					Tier:        known.tier,
					Description: fmt.Sprintf("AI Agent / Harness at ~/%s", folderName),
				})
			} else {
				// Unknown dot-directory (custom agent, harness, etc.)
				displayName := strings.Title(strings.TrimPrefix(folderName, ".")) + " (Harness/Agent)"
				agents = append(agents, AgentInfo{
					Name:        strings.TrimPrefix(folderName, "."),
					DisplayName: displayName,
					ConfigPath:  fullPath,
					ConfigType:  "folder",
					Exists:      true,
					HooksPath:   filepath.Join(fullPath, "hooks"),
					SkillsPath:  filepath.Join(fullPath, "skills"),
					Tier:        "unknown",
					Description: fmt.Sprintf("Custom dot-directory at ~/%s", folderName),
				})
			}
		}
	}

	// Also check ~/.config/ subdirs (like zed, opencode, gh)
	configDir := filepath.Join(homeDir, ".config")
	if configEntries, err := os.ReadDir(configDir); err == nil {
		for _, ce := range configEntries {
			if ce.IsDir() {
				name := ce.Name()
				fullPath := filepath.Join(configDir, name)
				if name == "zed" || name == "opencode" {
					agents = append(agents, AgentInfo{
						Name:        name,
						DisplayName: strings.Title(name) + " (in ~/.config)",
						ConfigPath:  fullPath,
						ConfigType:  "folder",
						Exists:      true,
						HooksPath:   filepath.Join(fullPath, "hooks"),
						SkillsPath:  filepath.Join(fullPath, "skills"),
						Tier:        "optional",
						Description: fmt.Sprintf("Editor/Agent at ~/.config/%s", name),
					})
				}
			}
		}
	}

	// Check custom paths if requested
	for _, cp := range customPaths {
		agents = append(agents, scanCustomPath(cp)...)
	}

	// Scan all drives if explicitly requested
	if scanAllDrives {
		agents = append(agents, scanDrives()...)
	}

	return agents
}

func scanCustomPath(customPath string) []AgentInfo {
	var agents []AgentInfo
	customPath = os.ExpandEnv(customPath)

	if info, err := os.Stat(customPath); err == nil && info.IsDir() {
		entries, _ := os.ReadDir(customPath)
		for _, entry := range entries {
			if entry.IsDir() {
				name := entry.Name()
				agents = append(agents, AgentInfo{
					Name:        "custom-" + strings.ToLower(name),
					DisplayName: "Custom: " + name,
					ConfigPath:  filepath.Join(customPath, name),
					ConfigType:  "folder",
					Exists:      true,
					HooksPath:   filepath.Join(customPath, name, "hooks"),
					SkillsPath:  filepath.Join(customPath, name, "skills"),
					Tier:        "custom",
					Description: fmt.Sprintf("Custom path: %s", customPath),
				})
			}
		}
	}
	return agents
}

func scanDrives() []AgentInfo {
	var agents []AgentInfo
	if runtime.GOOS == "windows" {
		drives := []string{"C:", "D:", "E:"}
		for _, drive := range drives {
			drivePath := drive + "\\"
			if _, err := os.Stat(drivePath); err == nil {
				common := []string{".claude", ".kimi", ".pi", ".omni", ".nimagent", "GodAgentOS", "NimAgentOS"}
				for _, folder := range common {
					fullPath := filepath.Join(drivePath, folder)
					if pathExists(fullPath) {
						agents = append(agents, AgentInfo{
							Name:        strings.ToLower(folder) + "-" + strings.ToLower(drive),
							DisplayName: fmt.Sprintf("%s (on %s)", strings.Title(strings.TrimPrefix(folder, ".")), drive),
							ConfigPath:  fullPath,
							ConfigType:  "folder",
							Exists:      true,
							HooksPath:   filepath.Join(fullPath, "hooks"),
							SkillsPath:  filepath.Join(fullPath, "skills"),
							Tier:        "drive",
							Description: fmt.Sprintf("Found on %s drive root", drive),
						})
					}
				}
			}
		}
	}
	return agents
}

func displayResults(agents []AgentInfo) {
	color.Cyan("\n📋 Discovered Agents & Harnesses (%d total):\n", len(agents))

	defaults := []AgentInfo{}
	optional := []AgentInfo{}
	unknown := []AgentInfo{}

	for _, a := range agents {
		switch a.Tier {
		case "default":
			defaults = append(defaults, a)
		case "optional":
			optional = append(optional, a)
		default:
			unknown = append(unknown, a)
		}
	}

	if len(defaults) > 0 {
		color.Green("▶ Default Coding Agents (%d):", len(defaults))
		for _, a := range defaults {
			fmt.Printf("  • %-26s %s\n", a.DisplayName, color.CyanString(a.ConfigPath))
		}
	}

	if len(optional) > 0 {
		color.Yellow("\n▶ Optional Coding Tools (%d):", len(optional))
		for _, a := range optional {
			fmt.Printf("  • %-26s %s\n", a.DisplayName, color.CyanString(a.ConfigPath))
		}
	}

	if len(unknown) > 0 {
		color.Magenta("\n▶ Other Harnesses & Dot-Directories (%d):", len(unknown))
		for _, a := range unknown {
			fmt.Printf("  • %-26s %s\n", a.DisplayName, color.CyanString(a.ConfigPath))
		}
	}

	fmt.Println()
}

func interactiveSelect(agents []AgentInfo) []AgentInfo {
	var items []string
	agentMap := make(map[string]AgentInfo)

	for _, agent := range agents {
		label := fmt.Sprintf("%s [%s] (%s)", agent.DisplayName, agent.Tier, agent.ConfigPath)
		items = append(items, label)
		agentMap[label] = agent
	}

	if len(items) == 0 {
		return []AgentInfo{}
	}

	return multiSelectPrompt(items, agentMap)
}

func multiSelectPrompt(items []string, agentMap map[string]AgentInfo) []AgentInfo {
	color.Yellow("\n📝 Enter numbers separated by commas (e.g., 1,3,5), 'defaults' for defaults, 'all' for everything, or ENTER to exit:")
	for i, item := range items {
		fmt.Printf("  %2d. %s\n", i+1, item)
	}

	prompt := promptui.Prompt{
		Label: "Selection",
		Validate: func(input string) error {
			if input == "all" || input == "defaults" || input == "" {
				return nil
			}
			parts := strings.Split(input, ",")
			for _, p := range parts {
				p = strings.TrimSpace(p)
				var val int
				if _, err := fmt.Sscanf(p, "%d", &val); err != nil {
					return fmt.Errorf("invalid number: %s", p)
				}
			}
			return nil
		},
	}

	result, err := prompt.Run()
	if err != nil {
		return []AgentInfo{}
	}

	result = strings.TrimSpace(result)
	var selected []AgentInfo

	if result == "all" {
		for _, item := range items {
			selected = append(selected, agentMap[item])
		}
	} else if result == "defaults" || result == "" {
		for _, item := range items {
			a := agentMap[item]
			if a.Tier == "default" {
				selected = append(selected, a)
			}
		}
	} else {
		parts := strings.Split(result, ",")
		for _, p := range parts {
			p = strings.TrimSpace(p)
			var idx int
			if _, err := fmt.Sscanf(p, "%d", &idx); err == nil && idx > 0 && idx <= len(items) {
				selected = append(selected, agentMap[items[idx-1]])
			}
		}
	}

	return selected
}

func saveSelection(agents []AgentInfo) {
	data, err := yaml.Marshal(agents)
	if err != nil {
		return
	}
	_ = os.WriteFile(".agent-selection.yaml", data, 0644)
	color.Green("Saved selection to .agent-selection.yaml")
}