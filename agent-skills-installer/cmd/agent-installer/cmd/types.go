package cmd

type AgentInfo struct {
	Name        string `json:"name" yaml:"name"`
	DisplayName string `json:"display_name" yaml:"display_name"`
	ConfigPath  string `json:"config_path" yaml:"config_path"`
	Exists      bool   `json:"exists" yaml:"exists"`
	Version     string `json:"version" yaml:"version"`
	Description string `json:"description" yaml:"description"`
	ConfigType  string `json:"config_type" yaml:"config_type"` // "folder", "file", "env"
	HooksPath   string `json:"hooks_path" yaml:"hooks_path"`
	SkillsPath  string `json:"skills_path" yaml:"skills_path"`
	Tier        string `json:"tier" yaml:"tier"` // "default", "optional", "unknown"
}

type SkillInfo struct {
	Name        string                 `json:"name" yaml:"name"`
	DisplayName string                 `json:"display_name" yaml:"display_name"`
	Path        string                 `json:"path" yaml:"path"`
	Category    string                 `json:"category" yaml:"category"`
	Description string                 `json:"description" yaml:"description"`
	Hooks       []string               `json:"hooks" yaml:"hooks"`
	Tools       []string               `json:"tools" yaml:"tools"`
	Config      map[string]interface{} `json:"config" yaml:"config"`
}

type InstallManifest struct {
	Version     string      `json:"version"`
	RepoRoot    string      `json:"repoRoot"`
	InstalledAt string      `json:"installedAt"`
	Agents      []AgentInfo `json:"agents"`
}
