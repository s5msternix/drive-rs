<script lang="ts">
	import '../app.css';
	import { onMount } from 'svelte';
	import { auth } from '$lib/api/client';
	import { user, isAuthenticated, clearAuth } from '$lib/stores/auth';
	import { goto } from '$app/navigation';
	import { page } from '$app/stores';

	let { children } = $props();

	onMount(async () => {
		const token = localStorage.getItem('token');
		if (token) {
			try {
				const userData = await auth.me();
				user.set(userData);
				isAuthenticated.set(true);
			} catch {
				clearAuth();
			}
		}
	});

	function logout() {
		clearAuth();
		goto('/login');
	}
</script>

{#if $isAuthenticated}
	<nav class="navbar">
		<div class="nav-left">
			<a href="/drive" class="logo">Drive-RS</a>
		</div>
		<div class="nav-center">
			<a href="/drive" class="nav-link" class:active={$page.url.pathname.startsWith('/drive')}>Dosyalar</a>
			<a href="/transfer" class="nav-link" class:active={$page.url.pathname.startsWith('/transfer')}>Transfer</a>
		</div>
		<div class="nav-right">
			<span class="username">{$user?.username}</span>
			<button class="btn-ghost" onclick={logout}>Çıkış</button>
		</div>
	</nav>
{/if}

<main>
	{@render children()}
</main>

<style>
	.navbar {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 0.75rem 1.5rem;
		background: var(--bg-secondary);
		border-bottom: 1px solid var(--border);
		position: sticky;
		top: 0;
		z-index: 100;
	}

	.logo {
		font-size: 1.25rem;
		font-weight: 700;
		color: var(--primary);
	}

	.nav-center {
		display: flex;
		gap: 0.25rem;
	}

	.nav-link {
		padding: 0.5rem 1rem;
		border-radius: var(--radius);
		color: var(--text-muted);
		font-size: 0.875rem;
		transition: all 0.15s;
	}

	.nav-link:hover, .nav-link.active {
		background: var(--bg-tertiary);
		color: var(--text);
	}

	.nav-right {
		display: flex;
		align-items: center;
		gap: 0.75rem;
	}

	.username {
		color: var(--text-muted);
		font-size: 0.875rem;
	}

	main {
		min-height: calc(100vh - 56px);
	}
</style>
