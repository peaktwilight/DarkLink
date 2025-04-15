package web

import (
	"net/http"
	"os"
	"path/filepath"
)

// StaticHandler manages static file serving
type StaticHandler struct {
	staticDir string
	webDir    string
}

// New creates a new static file handler
func New(staticDir string) *StaticHandler {
	return &StaticHandler{
		staticDir: staticDir,
		webDir:    filepath.Join(filepath.Dir(staticDir), "web"),
	}
}

// HandleRoot serves the root path and static files
func (h *StaticHandler) HandleRoot(w http.ResponseWriter, r *http.Request) {
	if r.URL.Path != "/" {
		// Serve other static files from the web directory
		if _, err := os.Stat(filepath.Join(h.webDir, r.URL.Path)); err == nil {
			http.ServeFile(w, r, filepath.Join(h.webDir, r.URL.Path))
			return
		}
		http.NotFound(w, r)
		return
	}
	http.ServeFile(w, r, filepath.Join(h.webDir, "index.html"))
}

// SetupStaticRoutes sets up routes for static file serving
func (h *StaticHandler) SetupStaticRoutes() {
	// Handle /static/ paths for backward compatibility
	fs := http.FileServer(http.Dir(h.staticDir))
	http.Handle("/static/", http.StripPrefix("/static/", fs))

	// Serve web assets from the web directory
	webFs := http.FileServer(http.Dir(h.webDir))
	http.Handle("/home/", http.StripPrefix("/home/", webFs))
}
