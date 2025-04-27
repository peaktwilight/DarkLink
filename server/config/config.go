package config

import (
	"fmt"
	"os"

	"gopkg.in/yaml.v3"
)

// LoadConfig loads the configuration from the specified YAML file
func LoadConfig(configPath string) (*Config, error) {
	// Ensure the config file exists
	if _, err := os.Stat(configPath); os.IsNotExist(err) {
		return nil, fmt.Errorf("config file does not exist: %s", configPath)
	}

	// Read the config file
	data, err := os.ReadFile(configPath)
	if err != nil {
		return nil, fmt.Errorf("error reading config file: %v", err)
	}

	// Parse the YAML
	config := &Config{}
	if err := yaml.Unmarshal(data, config); err != nil {
		return nil, fmt.Errorf("error parsing config file: %v", err)
	}

	// Validate and set defaults
	if err := validateConfig(config); err != nil {
		return nil, fmt.Errorf("config validation error: %v", err)
	}

	return config, nil
}

func validateConfig(config *Config) error {
	// Ensure required directories exist or can be created
	dirs := []string{config.Server.UploadDir, config.Server.StaticDir}
	for _, dir := range dirs {
		if dir == "" {
			continue
		}
		if err := os.MkdirAll(dir, 0755); err != nil {
			return fmt.Errorf("failed to create directory %s: %v", dir, err)
		}
	}

	// Validate protocol selection
	switch config.Communication.Protocol {
	case "http-polling", "dns-over-https", "socks5":
		// Valid protocols
	default:
		return fmt.Errorf("unsupported protocol: %s", config.Communication.Protocol)
	}

	// Set defaults if not specified
	if config.Server.Port == "" {
		config.Server.Port = "8080"
	}

	if config.Communication.HTTPPolling.HeartbeatInterval == 0 {
		config.Communication.HTTPPolling.HeartbeatInterval = 60
	}

	if config.Communication.DNSOverHTTPS.MaxChunkSize == 0 {
		config.Communication.DNSOverHTTPS.MaxChunkSize = 255
	}

	if config.Logging.Level == "" {
		config.Logging.Level = "info"
	}

	return nil
}
