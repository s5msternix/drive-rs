const API_BASE = '/api';

async function request<T>(path: string, options: RequestInit = {}): Promise<T> {
	const token = typeof window !== 'undefined' ? localStorage.getItem('token') : null;

	const headers: Record<string, string> = {
		...(options.headers as Record<string, string>)
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

	if (res.status === 204) return undefined as T;

	const contentType = res.headers.get('Content-Type') || '';
	if (contentType.includes('application/json')) {
		return res.json();
	}

	return res.blob() as unknown as T;
}

// Auth
export const auth = {
	register: (data: { email: string; username: string; password: string }) =>
		request<{ token: string; user: any }>('/auth/register', {
			method: 'POST',
			body: JSON.stringify(data)
		}),

	login: (data: { email: string; password: string }) =>
		request<{ token: string; user: any }>('/auth/login', {
			method: 'POST',
			body: JSON.stringify(data)
		}),

	me: () => request<any>('/auth/me')
};

// Files
export const files = {
	list: (folderId?: string) =>
		request<any[]>(`/files${folderId ? `?folder_id=${folderId}` : ''}`),

	upload: (file: File, folderId?: string) => {
		const formData = new FormData();
		formData.append('file', file);
		return request<any>(`/files/upload${folderId ? `?folder_id=${folderId}` : ''}`, {
			method: 'POST',
			body: formData
		});
	},

	download: (id: string) => request<Blob>(`/files/${id}`),

	delete: (id: string) => request<void>(`/files/${id}`, { method: 'DELETE' }),

	rename: (id: string, name: string) =>
		request<any>(`/files/${id}/rename`, {
			method: 'PATCH',
			body: JSON.stringify({ name })
		}),

	move: (id: string, folderId: string | null) =>
		request<any>(`/files/${id}/move`, {
			method: 'PUT',
			body: JSON.stringify({ folder_id: folderId })
		})
};

// Folders
export const folders = {
	list: (parentId?: string) =>
		request<any[]>(`/folders${parentId ? `?folder_id=${parentId}` : ''}`),

	create: (name: string, parentId?: string) =>
		request<any>('/folders', {
			method: 'POST',
			body: JSON.stringify({ name, parent_id: parentId || null })
		}),

	get: (id: string) => request<any>(`/folders/${id}`),

	rename: (id: string, name: string) =>
		request<any>(`/folders/${id}/rename`, {
			method: 'PATCH',
			body: JSON.stringify({ name })
		}),

	delete: (id: string) => request<void>(`/folders/${id}`, { method: 'DELETE' })
};

// Share
export const share = {
	createLink: (data: {
		file_id?: string;
		folder_id?: string;
		expires_in_hours?: number;
		max_downloads?: number;
		password?: string;
	}) =>
		request<{ id: string; token: string; url: string }>('/share', {
			method: 'POST',
			body: JSON.stringify(data)
		}),

	getInfo: (token: string) => request<any>(`/share/${token}`),

	download: (token: string) => request<Blob>(`/share/${token}/download`)
};

// Transfer (P2P)
export const transfer = {
	createSession: (data: { file_name: string; file_size: number }) =>
		request<{ id: string; token: string; file_name: string; file_size: number }>(
			'/transfer',
			{ method: 'POST', body: JSON.stringify(data) }
		),

	getSession: (token: string) =>
		request<{ id: string; token: string; file_name: string; file_size: number }>(
			`/transfer/${token}`
		),

	connectSignaling: (token: string): WebSocket => {
		const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
		return new WebSocket(`${protocol}//${window.location.host}/api/transfer/ws/${token}`);
	}
};
