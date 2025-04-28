package handlers

import (
	"errors"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"sync"
)

// FileTransfer represents an ongoing file transfer
type FileTransfer struct {
	ID       string
	Filename string
	Size     int64
	Received int64
	File     *os.File
}

// FileHandler manages file transfers for listeners
type FileHandler struct {
	uploadDir     string
	activeUploads map[string]*FileTransfer
	mu            sync.RWMutex
}

// NewFileHandler creates a new file handler instance
func NewFileHandler(uploadDir string) (*FileHandler, error) {
	if err := os.MkdirAll(uploadDir, 0755); err != nil {
		return nil, fmt.Errorf("failed to create upload directory: %v", err)
	}

	return &FileHandler{
		uploadDir:     uploadDir,
		activeUploads: make(map[string]*FileTransfer),
	}, nil
}

// StartUpload initializes a new file upload
func (h *FileHandler) StartUpload(transferID, filename string, size int64) (*FileTransfer, error) {
	h.mu.Lock()
	defer h.mu.Unlock()

	// Validate filename
	if filename == "" || filepath.Clean(filename) != filename {
		return nil, errors.New("invalid filename")
	}

	// Check if upload already exists
	if _, exists := h.activeUploads[transferID]; exists {
		return nil, errors.New("upload already in progress")
	}

	// Create file
	filepath := filepath.Join(h.uploadDir, filename)
	file, err := os.Create(filepath)
	if err != nil {
		return nil, fmt.Errorf("failed to create file: %v", err)
	}

	transfer := &FileTransfer{
		ID:       transferID,
		Filename: filename,
		Size:     size,
		File:     file,
	}

	h.activeUploads[transferID] = transfer
	return transfer, nil
}

// WriteChunk writes a chunk of data to an active upload
func (h *FileHandler) WriteChunk(transferID string, data []byte) (int64, error) {
	h.mu.Lock()
	transfer, exists := h.activeUploads[transferID]
	h.mu.Unlock()

	if !exists {
		return 0, errors.New("upload not found")
	}

	n, err := transfer.File.Write(data)
	if err != nil {
		return 0, fmt.Errorf("failed to write chunk: %v", err)
	}

	transfer.Received += int64(n)

	// Check if upload is complete
	if transfer.Received >= transfer.Size {
		if err := h.CompleteUpload(transferID); err != nil {
			return int64(n), fmt.Errorf("failed to complete upload: %v", err)
		}
	}

	return int64(n), nil
}

// CompleteUpload finalizes an upload
func (h *FileHandler) CompleteUpload(transferID string) error {
	h.mu.Lock()
	defer h.mu.Unlock()

	transfer, exists := h.activeUploads[transferID]
	if !exists {
		return errors.New("upload not found")
	}

	if err := transfer.File.Close(); err != nil {
		return fmt.Errorf("failed to close file: %v", err)
	}

	delete(h.activeUploads, transferID)
	return nil
}

// CancelUpload cancels and cleans up an upload
func (h *FileHandler) CancelUpload(transferID string) error {
	h.mu.Lock()
	defer h.mu.Unlock()

	transfer, exists := h.activeUploads[transferID]
	if !exists {
		return errors.New("upload not found")
	}

	if err := transfer.File.Close(); err != nil {
		return fmt.Errorf("failed to close file: %v", err)
	}

	if err := os.Remove(filepath.Join(h.uploadDir, transfer.Filename)); err != nil {
		return fmt.Errorf("failed to remove file: %v", err)
	}

	delete(h.activeUploads, transferID)
	return nil
}

// GetUpload retrieves information about an active upload
func (h *FileHandler) GetUpload(transferID string) (*FileTransfer, error) {
	h.mu.RLock()
	defer h.mu.RUnlock()

	transfer, exists := h.activeUploads[transferID]
	if !exists {
		return nil, errors.New("upload not found")
	}

	return transfer, nil
}

// ListUploads returns a list of all active uploads
func (h *FileHandler) ListUploads() []*FileTransfer {
	h.mu.RLock()
	defer h.mu.RUnlock()

	uploads := make([]*FileTransfer, 0, len(h.activeUploads))
	for _, transfer := range h.activeUploads {
		uploads = append(uploads, transfer)
	}

	return uploads
}

// DownloadFile retrieves a file from the upload directory
func (h *FileHandler) DownloadFile(filename string) (io.ReadCloser, error) {
	filepath := filepath.Join(h.uploadDir, filename)
	file, err := os.Open(filepath)
	if err != nil {
		return nil, fmt.Errorf("failed to open file: %v", err)
	}
	return file, nil
}

// DeleteFile removes a file from the upload directory
func (h *FileHandler) DeleteFile(filename string) error {
	filepath := filepath.Join(h.uploadDir, filename)
	if err := os.Remove(filepath); err != nil {
		return fmt.Errorf("failed to delete file: %v", err)
	}
	return nil
}

// This file will be moved to the new 'handlers' folder as 'file_handler.go'.
