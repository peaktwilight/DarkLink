package web

import (
	"net/http"
	"os"
	"path/filepath"
)

// New creates a new static file handler instance
//
// Pre-conditions:
//   - staticDir is a valid directory path containing static assets
//
// Post-conditions:
//   - Returns a properly configured StaticHandler instance
//   - webDir is set to the web subdirectory relative to staticDir
func New(staticDir string) (*StaticHandler, error) {
	exePath, err := os.Executable()
	if err != nil {
		return nil, err
	}
	exeDir := filepath.Dir(exePath)
	webDir := filepath.Join(exeDir, "web")
	return &StaticHandler{
		staticDir: staticDir,
		webDir:    webDir,
	}, nil
}

// HandleRoot serves the Vue SPA and static files
//
// Pre-conditions:
//   - Valid HTTP request and response writer
//   - webDir exists and contains necessary files
//
// Post-conditions:
//   - Serves index.html for root path (Vue SPA)
//   - Serves requested static files from web directory
//   - Returns 404 Not Found for non-existent files
func (h *StaticHandler) HandleRoot(w http.ResponseWriter, r *http.Request) {
	if r.URL.Path == "/" {
		// Serve index.html for Vue SPA
		http.ServeFile(w, r, filepath.Join(h.webDir, "index.html"))
		return
	}
	
	// Serve other static files from the web directory
	if _, err := os.Stat(filepath.Join(h.webDir, r.URL.Path)); err == nil {
		http.ServeFile(w, r, filepath.Join(h.webDir, r.URL.Path))
		return
	}
	http.NotFound(w, r)
}

// SetupStaticRoutes sets up routes for static file serving
//
// Pre-conditions:
//   - staticDir and webDir exist and contain necessary files
//
// Post-conditions:
//   - Routes are registered with the HTTP server
//   - /static/ paths are served from staticDir
func (h *StaticHandler) SetupStaticRoutes() {
	// Handle /static/ paths for backward compatibility
	fs := http.FileServer(http.Dir(h.staticDir))
	http.Handle("/static/", http.StripPrefix("/static/", fs))
}
