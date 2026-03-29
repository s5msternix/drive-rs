import { writable } from 'svelte/store';

interface User {
	id: string;
	email: string;
	username: string;
	storage_used: number;
	storage_limit: number;
}

export const user = writable<User | null>(null);
export const isAuthenticated = writable(false);

export function setAuth(token: string, userData: User) {
	localStorage.setItem('token', token);
	user.set(userData);
	isAuthenticated.set(true);
}

export function clearAuth() {
	localStorage.removeItem('token');
	user.set(null);
	isAuthenticated.set(false);
}

export function formatBytes(bytes: number): string {
	if (bytes === 0) return '0 B';
	const k = 1024;
	const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
	const i = Math.floor(Math.log(bytes) / Math.log(k));
	return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}
