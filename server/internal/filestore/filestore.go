package filestore

import (
	"io"
	"net/http"
	"os"
	"path/filepath"
	"strings"
	"time"
)

// FileStore handles file operations and storage
type FileStore struct {
	baseDir string
}

type FileInfo struct {
	Name     string `json:"name"`
	Size     int64  `json:"size"`
	Modified string `json:"modified"`
}

// New creates a new FileStore instance
func New(baseDir string) (*FileStore, error) {
	if err := os.MkdirAll(baseDir, 0755); err != nil {
		return nil, err
	}
	return &FileStore{baseDir: baseDir}, nil
}

// HandleUpload handles file upload requests
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

// ServeFile serves a file for download
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
func (fs *FileStore) DeleteFile(fileName string) error {
	// Prevent directory traversal
	if strings.Contains(fileName, "..") {
		return os.ErrNotExist
	}

	return os.Remove(filepath.Join(fs.baseDir, fileName))
}
