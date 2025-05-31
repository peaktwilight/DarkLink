package web

// StaticHandler manages static file serving for the web interface
// It provides methods for serving HTML, CSS, JavaScript, and other static files
// used by the DarkLink web console.
type StaticHandler struct {
	staticDir string
	webDir    string
}
