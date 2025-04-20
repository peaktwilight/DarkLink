class FileDropManager {
    constructor() {
        this.dropZone = document.getElementById('dropZone');
        this.fileInput = document.getElementById('fileInput');
        this.uploadProgress = document.getElementById('uploadProgress');
        this.progressBarFill = document.getElementById('progressBarFill');

        this.setupEventListeners();
        this.loadFiles();
        this.startAutoRefresh();
    }

    setupEventListeners() {
        this.dropZone.addEventListener('dragover', (e) => {
            e.preventDefault();
            this.dropZone.classList.add('drag-over');
        });

        this.dropZone.addEventListener('dragleave', () => {
            this.dropZone.classList.remove('drag-over');
        });

        this.dropZone.addEventListener('drop', (e) => {
            e.preventDefault();
            this.dropZone.classList.remove('drag-over');
            const files = e.dataTransfer.files;
            if (files.length > 0) {
                this.uploadFiles(files);
            }
        });

        // Connect the browse button to the hidden file input
        const browseButton = document.querySelector('.browse-button');
        if (browseButton) {
            browseButton.addEventListener('click', (e) => {
                e.preventDefault();
                this.fileInput.click();
            });
        }

        this.fileInput.addEventListener('change', (e) => {
            if (e.target.files.length > 0) {
                this.uploadFiles(e.target.files);
            }
        });
    }

    async uploadFiles(files) {
        const formData = new FormData();
        for (let i = 0; i < files.length; i++) {
            formData.append('files', files[i]);
        }

        this.uploadProgress.style.display = 'block';
        this.progressBarFill.style.width = '0%';

        try {
            const xhr = new XMLHttpRequest();
            xhr.open('POST', '/api/file_drop/upload', true);

            xhr.upload.onprogress = (e) => {
                if (e.lengthComputable) {
                    const percentComplete = (e.loaded / e.total) * 100;
                    this.progressBarFill.style.width = percentComplete + '%';
                }
            };

            xhr.onload = () => {
                if (xhr.status === 200) {
                    this.loadFiles();
                    this.fileInput.value = '';
                    setTimeout(() => {
                        this.uploadProgress.style.display = 'none';
                    }, 1000);
                } else {
                    throw new Error('Upload failed');
                }
            };

            xhr.onerror = () => {
                throw new Error('Upload failed');
            };

            xhr.send(formData);
        } catch (error) {
            console.error('Error uploading files:', error);
            alert('Failed to upload files: ' + error.message);
            this.uploadProgress.style.display = 'none';
        }
    }

    async loadFiles() {
        try {
            const response = await fetch('/api/file_drop/list');
            if (!response.ok) {
                throw new Error(`Server returned ${response.status}: ${response.statusText}`);
            }
            
            const text = await response.text();
            let files;
            try {
                files = JSON.parse(text);
            } catch (e) {
                throw new Error('Invalid server response format');
            }

            if (!Array.isArray(files)) {
                throw new Error('Server returned invalid data format');
            }
            
            const fileListBody = document.getElementById('fileListBody');
            
            if (files.length === 0) {
                fileListBody.innerHTML = `
                    <tr>
                        <td colspan="4">
                            <div class="empty-state">
                                <p>No files uploaded yet</p>
                            </div>
                        </td>
                    </tr>
                `;
                return;
            }

            fileListBody.innerHTML = '';
            files.forEach(file => {
                const row = document.createElement('tr');
                row.innerHTML = `
                    <td>${this.escapeHtml(file.name)}</td>
                    <td>${this.formatBytes(file.size)}</td>
                    <td>${new Date(file.modified).toLocaleString()}</td>
                    <td class="file-actions">
                        <button onclick="fileDropManager.downloadFile('${encodeURIComponent(file.name)}')">Download</button>
                        <button class="delete" onclick="fileDropManager.deleteFile('${encodeURIComponent(file.name)}')">Delete</button>
                    </td>
                `;
                fileListBody.appendChild(row);
            });
        } catch (error) {
            console.error('Error loading files:', error);
            document.getElementById('fileListBody').innerHTML = `
                <tr>
                    <td colspan="4">
                        <div class="empty-state">
                            <p>Error loading files: ${this.escapeHtml(error.message)}</p>
                        </div>
                    </td>
                </tr>
            `;
        }
    }

    downloadFile(filename) {
        window.location.href = `/api/file_drop/download/${filename}`;
    }

    async deleteFile(filename) {
        if (!confirm('Are you sure you want to delete this file?')) {
            return;
        }

        try {
            const response = await fetch(`/api/file_drop/delete/${filename}`, {
                method: 'DELETE'
            });

            if (response.ok) {
                this.loadFiles();
            } else {
                throw new Error('Failed to delete file');
            }
        } catch (error) {
            console.error('Error deleting file:', error);
            alert('Failed to delete file: ' + error.message);
        }
    }

    formatBytes(bytes) {
        if (bytes === 0) return '0 Bytes';
        const k = 1024;
        const sizes = ['Bytes', 'KB', 'MB', 'GB'];
        const i = Math.floor(Math.log(bytes) / Math.log(k));
        return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
    }

    escapeHtml(unsafe) {
        return unsafe
            .replace(/&/g, "&amp;")
            .replace(/</g, "&lt;")
            .replace(/>/g, "&gt;")
            .replace(/"/g, "&quot;")
            .replace(/'/g, "&#039;");
    }

    startAutoRefresh() {
        // Refresh file list every 30 seconds
        setInterval(() => this.loadFiles(), 30000);
    }
}

// Initialize the file drop manager when the page loads
let fileDropManager;
document.addEventListener('DOMContentLoaded', () => {
    fileDropManager = new FileDropManager();
});