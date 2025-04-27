package config

type Config struct {
	Server struct {
		Port      string `yaml:"port"`
		UploadDir string `yaml:"uploadDir"`
		StaticDir string `yaml:"staticDir"`
	} `yaml:"server"`

	Communication struct {
		Protocol    string `yaml:"protocol"`
		HTTPPolling struct {
			HeartbeatInterval int `yaml:"heartbeatInterval"`
		} `yaml:"http-polling"`
	} `yaml:"communication"`

	Security struct {
		EnableCORS  bool     `yaml:"enableCORS"`
		CORSOrigins []string `yaml:"corsOrigins"`
	} `yaml:"security"`

	Logging struct {
		Level string `yaml:"level"`
		File  string `yaml:"file"`
	} `yaml:"logging"`
}
