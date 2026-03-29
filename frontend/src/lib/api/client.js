const API_BASE = '/api';

async function request(path, options = {}) {
	const token = typeof window !== 'undefined' ? localStorage.getItem('token') : null;

	const headers = {
		...(options.headers || {})
	};

	if (token) {
		headers['Authorization'] = `Bearer ${token}`;
	}

	// Don't set Content-Type for FormData
	if (!(options.body instanceof FormData)) {
		headers['Content-Type'] = 'application/json';
	}

	const res = await fetch(`${API_BASE}${path}`, {
		...options,
		headers
	});

	if (!res.ok) {
		if (res.status === 401) {
			localStorage.removeItem('token');
			window.location.href = '/login';
		}
		throw new Error(`HTTP ${res.status}: ${res.statusText}`);
	}

	if (res.status === 204) return undefined;

	const contentType = res.headers.get('Content-Type') || '';
	if (contentType.includes('application/json')) {
		return res.json();
	}

	return res.blob();
}

// Auth
export const auth = {
	register: (data) =>
		request('/auth/register', {
			method: 'POST',
			body: JSON.stringify(data)
		}),

	login: (data) =>
		request('/auth/login', {
			method: 'POST',
			body: JSON.stringify(data)
		}),

	me: () => request('/auth/me')
};

// Files
export const files = {
	list: (folderId) =>
		request(`/files${folderId ? `?folder_id=${folderId}` : ''}`),

	upload: (file, folderId) => {
		const formData = new FormData();
		formData.append('file', file);
		return request(`/files/upload${folderId ? `?folder_id=${folderId}` : ''}`, {
			method: 'POST',
			body: formData
		});
	},

	download: (id) => request(`/files/${id}`),

	delete: (id) => request(`/files/${id}`, { method: 'DELETE' }),

	rename: (id, name) =>
		request(`/files/${id}/rename`, {
			method: 'PATCH',
			body: JSON.stringify({ name })
		}),

	move: (id, folderId) =>
		request(`/files/${id}/move`, {
			method: 'PUT',
			body: JSON.stringify({ folder_id: folderId })
		})
};

// Folders
export const folders = {
	list: (parentId) =>
		request(`/folders${parentId ? `?folder_id=${parentId}` : ''}`),

	create: (name, parentId) =>
		request('/folders', {
			method: 'POST',
			body: JSON.stringify({ name, parent_id: parentId || null })
		}),

	get: (id) => request(`/folders/${id}`),

	rename: (id, name) =>
		request(`/folders/${id}/rename`, {
			method: 'PATCH',
			body: JSON.stringify({ name })
		}),

	delete: (id) => request(`/folders/${id}`, { method: 'DELETE' })
};

// Share
export const share = {
	createLink: (data) =>
		request('/share', {
			method: 'POST',
			body: JSON.stringify(data)
		}),

	getInfo: (token) => request(`/share/${token}`),

	download: (token) => request(`/share/${token}/download`)
};

// Transfer (P2P)
export const transfer = {
	createSession: (data) =>
		request('/transfer', { method: 'POST', body: JSON.stringify(data) }),

	getSession: (token) => request(`/transfer/${token}`),

	connectSignaling: (token) => {
		const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
		return new WebSocket(`${protocol}//${window.location.host}/api/transfer/ws/${token}`);
	}
};
