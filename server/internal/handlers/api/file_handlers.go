package api

import (
	"encoding/json"
	"microc2/server/internal/filestore"
	"microc2/server/internal/handlers/api/payload"
	"microc2/server/internal/protocols"
	"net/http"
	"strings"
)

// FileHandlers manages file-related HTTP endpoints
type FileHandlers struct {
	fileStore *filestore.FileStore
}

// NewFileHandlers creates a new file handlers instance
func NewFileHandlers(store *filestore.FileStore) *FileHandlers {
	return &FileHandlers{fileStore: store}
}

// HandleFileUpload handles file upload requests
func (h *FileHandlers) HandleFileUpload(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	if err := h.fileStore.HandleUpload(r); err != nil {
		http.Error(w, "Failed to handle file upload", http.StatusInternalServerError)
		return
	}

	w.WriteHeader(http.StatusOK)
}

// HandleFileList handles file listing requests
func (h *FileHandlers) HandleFileList(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	files, err := h.fileStore.ListFiles()
	if err != nil {
		http.Error(w, "Failed to list files", http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(files)
}

// HandleFileDownload handles file download requests
func (h *FileHandlers) HandleFileDownload(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	filename := strings.TrimPrefix(r.URL.Path, "/api/file_drop/download/")
	if filename == "" {
		http.Error(w, "No filename provided", http.StatusBadRequest)
		return
	}

	if err := h.fileStore.ServeFile(filename, w, r); err != nil {
		http.Error(w, "File not found", http.StatusNotFound)
		return
	}
}

// HandleFileDelete handles file deletion requests
func (h *FileHandlers) HandleFileDelete(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodDelete {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	filename := strings.TrimPrefix(r.URL.Path, "/api/file_drop/delete/")
	if filename == "" {
		http.Error(w, "No filename provided", http.StatusBadRequest)
		return
	}

	if err := h.fileStore.DeleteFile(filename); err != nil {
		http.Error(w, "Failed to delete file", http.StatusInternalServerError)
		return
	}

	w.WriteHeader(http.StatusOK)
}

// PayloadHandlerSetup creates and initializes a new payload handler
func PayloadHandlerSetup(payloadsDir, agentSourceDir string, listenerManager *protocols.ListenerManager) *payload.PayloadHandler {
	adapter := payload.NewListenerManagerAdapter(listenerManager)
	handler := payload.NewPayloadHandler(payloadsDir, agentSourceDir, adapter)
	return handler
}
