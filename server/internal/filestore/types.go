package filestore

// FileStore handles file operations and storage for the application
// It provides methods for uploading, listing, serving, and deleting files
// within a specified base directory.
type FileStore struct {
	baseDir string
}

// FileInfo represents metadata about a file in the store
// Used for listing files and providing information to clients
type FileInfo struct {
	Name     string `json:"name"`
	Size     int64  `json:"size"`
	Modified string `json:"modified"`
}
