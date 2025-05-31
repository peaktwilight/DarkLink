package api

import (
	"encoding/json"
	"darklink/server/internal/filestore"
	"darklink/server/internal/handlers/api/payload"
	"darklink/server/internal/listeners" // Updated from `networking`
	"net/http"
	"os"
	"strings"
)

// NewFileHandlers creates a new file handlers instance
//
// Pre-conditions:
//   - fileStore is a properly initialized FileStore instance
//
// Post-conditions:
//   - Returns a configured FileHandlers instance ready to handle HTTP requests
func NewFileHandlers(fileStore *filestore.FileStore) *FileHandlers {
	return &FileHandlers{
		fileStore: fileStore,
	}
}

// HandleFileUpload processes file upload requests
//
// Pre-conditions:
//   - Request is a POST multipart/form/data request
//   - Request contains one or more files in the "files" field
//
// Post-conditions:
//   - Uploaded files are saved to the file store
//   - Returns 200 OK on success
//   - Returns appropriate error status on failure
func (h *FileHandlers) HandleFileUpload(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	err := h.fileStore.HandleUpload(r)
	if err != nil {
		http.Error(w, "Failed to upload file: "+err.Error(), http.StatusInternalServerError)
		return
	}

	w.WriteHeader(http.StatusOK)
}

// HandleFileList returns a list of files in the file store
//
// Pre-conditions:
//   - Request is a GET request
//
// Post-conditions:
//   - Response contains a JSON array of file information objects
//   - Each object includes file name, size, and modification time
//   - Returns appropriate error status on failure
func (h *FileHandlers) HandleFileList(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	files, err := h.fileStore.ListFiles()
	if err != nil {
		http.Error(w, "Failed to list files: "+err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(files)
}

// HandleFileDownload serves a file for download
//
// Pre-conditions:
//   - Request is a GET request
//   - Request URL contains the file name after "/api/file_drop/download/"
//
// Post-conditions:
//   - Requested file is served for download if it exists
//   - Appropriate Content-Disposition and Content-Type headers are set
//   - Returns 404 Not Found if the file doesn't exist
//   - Returns appropriate error status on other failures
func (h *FileHandlers) HandleFileDownload(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	fileName := strings.TrimPrefix(r.URL.Path, "/api/file_drop/download/")
	if fileName == "" {
		http.Error(w, "File name is required", http.StatusBadRequest)
		return
	}

	err := h.fileStore.ServeFile(fileName, w, r)
	if err != nil {
		http.Error(w, "File not found", http.StatusNotFound)
		return
	}
}

// HandleFileDelete deletes a file from the file store
//
// Pre-conditions:
//   - Request is a DELETE request
//   - Request URL contains the file name after "/api/file_drop/delete/"
//
// Post-conditions:
//   - Requested file is deleted if it exists
//   - Returns 200 OK on success
//   - Returns 404 Not Found if the file doesn't exist
//   - Returns appropriate error status on other failures
func (h *FileHandlers) HandleFileDelete(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodDelete {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	fileName := strings.TrimPrefix(r.URL.Path, "/api/file_drop/delete/")
	if fileName == "" {
		http.Error(w, "File name is required", http.StatusBadRequest)
		return
	}

	err := h.fileStore.DeleteFile(fileName)
	if err != nil {
		if err == os.ErrNotExist {
			http.Error(w, "File not found", http.StatusNotFound)
		} else {
			http.Error(w, "Failed to delete file: "+err.Error(), http.StatusInternalServerError)
		}
		return
	}

	w.WriteHeader(http.StatusOK)
}

// PayloadHandlerSetup creates and initializes a new payload handler
func PayloadHandlerSetup(payloadsDir, agentSourceDir string, _ *listeners.ListenerManager) *payload.PayloadHandler {
	return payload.NewPayloadHandler(payloadsDir, agentSourceDir)
}
