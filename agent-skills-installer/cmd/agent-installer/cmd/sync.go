package cmd

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"sort"
	"strings"
	"time"

	"github.com/fatih/color"
	"github.com/spf13/cobra"
)

type CatalogEntry struct {
	Name        string `json:"name"`
	Path        string `json:"path"`
	Category    string `json:"category"`
	Description string `json:"description"`
	IsNested    bool   `json:"is_nested"`
}

type CatalogJson struct {
	Generated   bool           `json:"generated"`
	GeneratedAt string         `json:"generatedAt"`
	Source      string         `json:"source"`
	Count       int            `json:"count"`
	NestedCount int            `json:"nestedCount"`
	Skills      []CatalogEntry `json:"skills"`
}

var syncCmd = &cobra.Command{
	Use:   "sync",
	Short: "Scan skills/ folder and generate skills.json and INDEX.md natively",
	Long:  `Recursively scans all skills in the repository and generates skills.json catalog and INDEX.md.`,
	RunE:  runSync,
}

func init() {
	rootCmd.AddCommand(syncCmd)
}

func runSync(cmd *cobra.Command, args []string) error {
	repoRoot := getRepoRoot()
	skillsDir := filepath.Join(repoRoot, "skills")

	color.Cyan("🔄 Syncing skills catalog from %s...\n", skillsDir)

	if !pathExists(skillsDir) {
		return fmt.Errorf("skills directory not found at %s", skillsDir)
	}

	var topLevel []CatalogEntry
	var nested []CatalogEntry

	err := filepath.Walk(skillsDir, func(path string, info os.FileInfo, err error) error {
		if err != nil {
			return nil
		}
		if info.IsDir() {
			return nil
		}

		if strings.EqualFold(info.Name(), "SKILL.md") {
			skillFolder := filepath.Dir(path)
			rel, _ := filepath.Rel(skillsDir, skillFolder)
			parts := strings.Split(filepath.ToSlash(rel), "/")

			name := filepath.Base(skillFolder)
			category := "general"
			isNested := false

			if len(parts) > 1 {
				category = parts[0]
				isNested = true
			}

			// Read description from SKILL.md
			desc := extractSkillDescription(path)

			entry := CatalogEntry{
				Name:        name,
				Path:        filepath.ToSlash(rel),
				Category:    category,
				Description: desc,
				IsNested:    isNested,
			}

			if isNested {
				nested = append(nested, entry)
			} else {
				topLevel = append(topLevel, entry)
			}
		}
		return nil
	})

	if err != nil {
		return err
	}

	// Sort alphabetically
	sort.Slice(topLevel, func(i, j int) bool { return topLevel[i].Name < topLevel[j].Name })
	sort.Slice(nested, func(i, j int) bool { return nested[i].Name < nested[j].Name })

	allSkills := append([]CatalogEntry{}, topLevel...)
	allSkills = append(allSkills, nested...)

	catalog := CatalogJson{
		Generated:   true,
		GeneratedAt: time.Now().UTC().Format(time.RFC3339),
		Source:      filepath.ToSlash(skillsDir),
		Count:       len(topLevel),
		NestedCount: len(nested),
		Skills:      allSkills,
	}

	// Write skills.json
	skillsJsonPath := filepath.Join(repoRoot, "skills.json")
	data, err := json.MarshalIndent(catalog, "", "  ")
	if err != nil {
		return err
	}

	if err := os.WriteFile(skillsJsonPath, data, 0644); err != nil {
		return err
	}
	color.Green("✓ Generated %s (%d top-level, %d nested skills)", skillsJsonPath, len(topLevel), len(nested))

	// Write skills/INDEX.md
	indexMdPath := filepath.Join(skillsDir, "INDEX.md")
	var sb strings.Builder
	sb.WriteString("# ⚡ Skills Index\n\n")
	sb.WriteString(fmt.Sprintf("Auto-generated on %s\n\n", time.Now().Format("2006-01-02 15:04:05")))
	sb.WriteString(fmt.Sprintf("Total Skills: **%d** (%d top-level, %d library skills)\n\n", len(allSkills), len(topLevel), len(nested)))

	sb.WriteString("## Top-Level Skills\n\n")
	sb.WriteString("| Skill | Category | Description |\n")
	sb.WriteString("| :--- | :--- | :--- |\n")
	for _, s := range topLevel {
		sb.WriteString(fmt.Sprintf("| **%s** | `%s` | %s |\n", s.Name, s.Category, s.Description))
	}

	if len(nested) > 0 {
		sb.WriteString("\n## Nested & Category Skills\n\n")
		sb.WriteString("| Skill | Path | Category | Description |\n")
		sb.WriteString("| :--- | :--- | :--- | :--- |\n")
		for _, s := range nested {
			sb.WriteString(fmt.Sprintf("| **%s** | `%s` | `%s` | %s |\n", s.Name, s.Path, s.Category, s.Description))
		}
	}

	_ = os.WriteFile(indexMdPath, []byte(sb.String()), 0644)
	color.Green("✓ Generated %s", indexMdPath)

	return nil
}

func extractSkillDescription(skillMdPath string) string {
	content, err := os.ReadFile(skillMdPath)
	if err != nil {
		return ""
	}
	lines := strings.Split(string(content), "\n")
	inFrontmatter := false
	for _, line := range lines {
		trimmed := strings.TrimSpace(line)
		if trimmed == "---" {
			inFrontmatter = !inFrontmatter
			continue
		}
		if inFrontmatter && strings.HasPrefix(strings.ToLower(trimmed), "description:") {
			desc := strings.TrimPrefix(trimmed, "description:")
			desc = strings.TrimPrefix(desc, "Description:")
			return strings.Trim(strings.TrimSpace(desc), "\"'")
		}
		if !inFrontmatter && trimmed != "" && !strings.HasPrefix(trimmed, "#") {
			if len(trimmed) > 120 {
				return trimmed[:117] + "..."
			}
			return trimmed
		}
	}
	return ""
}
