# DarkLink Modern Frontend

A modern Vue.js 3 frontend for the DarkLink C2 server with enhanced UX, component-based architecture, and sleek dark theme.

## ✨ Features

### 🏗️ **Modern Architecture**
- **Vue.js 3** with Composition API
- **Component-based** design for maintainability
- **Vite** for fast development and optimized builds
- **TypeScript-ready** structure

### 🎨 **Enhanced UI/UX**
- **GitHub Dark-inspired** theme
- **Responsive design** for all screen sizes
- **Real-time updates** with WebSocket connectivity
- **Loading states** and **error handling**
- **Keyboard shortcuts** and **accessibility**

### 🚀 **Advanced Features**
- **Auto-reconnecting** WebSocket connections
- **Command history** with arrow key navigation
- **Tab completion** in terminal
- **Drag & drop** file uploads
- **Progress indicators** for long operations
- **Status notifications** with auto-dismiss

## 🛠️ Development

### Prerequisites
- Node.js 16+ 
- npm or yarn

### Setup
```bash
# Install dependencies
npm install

# Start development server
npm run dev

# Build for production
npm run build

# Preview production build
npm run preview
```

### Development URLs
- **Dashboard**: http://localhost:3000/
- **Listeners**: http://localhost:3000/listeners
- **Payload Generator**: http://localhost:3000/payload
- **File Drop**: http://localhost:3000/file-drop
- **Terminal**: http://localhost:3000/terminal

## 📁 Project Structure

```
src/
├── assets/           # Static assets and styles
├── components/       # Reusable Vue components
│   ├── ui/          # Base UI components (Button, Card, etc.)
│   ├── dashboard/   # Dashboard-specific components
│   ├── listeners/   # Listener management components
│   ├── payload/     # Payload generation components
│   ├── fileDrop/    # File upload/download components
│   ├── terminal/    # Terminal interface components
│   └── layout/      # Layout components (Sidebar, etc.)
├── composables/     # Vue composables for shared logic
├── router/          # Vue Router configuration
└── views/           # Page-level components
```

## 🎯 Component Architecture

### **Base UI Components**
- `Button` - Configurable buttons with variants and loading states
- `Card` - Container component with header/content/actions slots
- `Icon` - SVG icon system with 20+ icons
- `StatusMessage` - Notification component with auto-dismiss

### **Dashboard Components**
- `AgentsList` - Interactive agent management
- `ListenersList` - Listener status and controls
- `EventLog` - Real-time server events with filtering
- `CommandShell` - Terminal-like command interface

### **Feature Components**
- `ListenerForm` - Comprehensive listener configuration
- `PayloadForm` - Advanced payload generation options
- `FileUpload` - Drag & drop file upload with progress
- `TerminalInterface` - Full terminal emulation

## 🔌 API Integration

### **HTTP API**
- Automatic error handling and retries
- Loading states and progress tracking
- Consistent response formatting

### **WebSocket Connections**
- Auto-reconnecting with exponential backoff
- Real-time log streaming
- Terminal session management
- Connection status indicators

## 🎨 Theming

### **CSS Variables**
All colors and spacing use CSS custom properties for easy theming:

```css
:root {
  --bg-color: #0d1117;
  --text-color: #f0f6fc;
  --accent-color: #58a6ff;
  --success-color: #3fb950;
  --error-color: #f85149;
  /* ... */
}
```

### **Component Styling**
- Scoped CSS for component isolation
- Utility classes for rapid development
- Responsive design with mobile-first approach

## 🚦 Usage

### **For Development**
1. Start the Go server: `go run ./cmd/server.go`
2. Start the frontend: `npm run dev`
3. Access at http://localhost:3000

### **For Production**
1. Build the frontend: `npm run build`
2. The Go server serves the built files from `/dist`
3. Access through the Go server port

## 🔧 Configuration

### **Proxy Setup**
The development server proxies API calls to the Go server:

```js
// vite.config.js
server: {
  proxy: {
    '/api': 'http://localhost:8080',
    '/ws': {
      target: 'ws://localhost:8080',
      ws: true
    }
  }
}
```

### **Build Configuration**
Multi-page setup for backwards compatibility:

```js
build: {
  rollupOptions: {
    input: {
      main: 'index.html',
      listeners: 'listeners.html',
      payload: 'payload.html',
      fileDrop: 'file_drop.html',
      terminal: 'server_terminal.html'
    }
  }
}
```

## 🏆 Improvements Over Original

### **Code Quality**
- ✅ Component-based architecture vs monolithic files
- ✅ TypeScript-ready structure vs plain JavaScript
- ✅ Modern ES6+ syntax vs legacy code
- ✅ Reactive state management vs manual DOM manipulation

### **User Experience**
- ✅ Loading states and error handling
- ✅ Real-time updates without page refresh
- ✅ Keyboard shortcuts and accessibility
- ✅ Mobile-responsive design
- ✅ Consistent styling and interactions

### **Developer Experience**
- ✅ Hot reload development server
- ✅ Component-scoped styling
- ✅ Reusable composables for shared logic
- ✅ Automatic code splitting and optimization

## 🔮 Future Enhancements

- **Dark/Light theme toggle**
- **Advanced filtering and search**
- **Bulk operations for agents/listeners**
- **Real-time charts and metrics**
- **Plugin system for extensions**
- **Offline support with service workers**

---

**Built with ❤️ using Vue.js 3, Vite, and modern web standards**