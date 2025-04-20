package filestore

import (
	"io"
	"net/http"
	"os"
	"path/filepath"
	"strings"
	"time"
)

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

// New creates a new FileStore instance
//
// Pre-conditions:
//   - baseDir is a valid directory path
//
// Post-conditions:
//   - Returns an initialized FileStore instance
//   - Creates the base directory if it doesn't exist
//   - Returns an error if directory creation fails
func New(baseDir string) (*FileStore, error) {
	if err := os.MkdirAll(baseDir, 0755); err != nil {
		return nil, err
	}
	return &FileStore{baseDir: baseDir}, nil
}

// HandleUpload handles file upload requests from HTTP
//
// Pre-conditions:
//   - Request contains a valid multipart form with files
//   - Request content size is within the limit (32MB)
//
// Post-conditions:
//   - Files are saved to the store's base directory
//   - Returns an error if parsing or file operations fail
func (fs *FileStore) HandleUpload(r *http.Request) error {
	err := r.ParseMultipartForm(32 << 20) // 32MB max memory
	if err != nil {
		return err
	}

	files := r.MultipartForm.File["files"]
	for _, fileHeader := range files {
		file, err := fileHeader.Open()
		if err != nil {
			return err
		}
		defer file.Close()

		// Create the destination file
		dst, err := os.Create(filepath.Join(fs.baseDir, fileHeader.Filename))
		if err != nil {
			return err
		}
		defer dst.Close()

		// Copy the uploaded file to the destination
		if _, err := io.Copy(dst, file); err != nil {
			return err
		}
	}

	return nil
}

// ListFiles returns a list of files in the store
//
// Pre-conditions:
//   - BaseDir exists or can be created
//
// Post-conditions:
//   - Returns a slice of FileInfo structs for all files in the directory
//   - Returns an error if the directory can't be read
func (fs *FileStore) ListFiles() ([]FileInfo, error) {
	if err := os.MkdirAll(fs.baseDir, 0755); err != nil {
		return nil, err
	}

	files, err := os.ReadDir(fs.baseDir)
	if err != nil {
		return nil, err
	}

	fileList := make([]FileInfo, 0)
	for _, file := range files {
		info, err := file.Info()
		if err != nil {
			continue
		}
		fileList = append(fileList, FileInfo{
			Name:     info.Name(),
			Size:     info.Size(),
			Modified: info.ModTime().Format(time.RFC3339),
		})
	}

	return fileList, nil
}

// ServeFile serves a file for download via HTTP
//
// Pre-conditions:
//   - fileName is a valid file name without directory traversal characters
//   - File exists in the base directory
//
// Post-conditions:
//   - File is served to the HTTP response writer
//   - Returns an error if file doesn't exist or path is invalid
func (fs *FileStore) ServeFile(fileName string, w http.ResponseWriter, r *http.Request) error {
	// Prevent directory traversal
	if strings.Contains(fileName, "..") {
		return os.ErrNotExist
	}

	filePath := filepath.Join(fs.baseDir, fileName)
	http.ServeFile(w, r, filePath)
	return nil
}

// DeleteFile deletes a file from the store
//
// Pre-conditions:
//   - fileName is a valid file name without directory traversal characters
//   - File exists in the base directory
//
// Post-conditions:
//   - File is deleted from the filesystem
//   - Returns an error if deletion fails or path is invalid
func (fs *FileStore) DeleteFile(fileName string) error {
	// Prevent directory traversal
	if strings.Contains(fileName, "..") {
		return os.ErrNotExist
	}

	return os.Remove(filepath.Join(fs.baseDir, fileName))
}
