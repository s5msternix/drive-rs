<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { share as shareApi } from '$lib/api/client';
	import { formatBytes } from '$lib/stores/auth';

	let info: any = $state(null);
	let error = $state('');
	let downloading = $state(false);

	const token = $page.params.token!;

	onMount(async () => {
		try {
			info = await shareApi.getInfo(token!);
		} catch (e: any) {
			error = 'Bu paylaşım linki geçersiz veya süresi dolmuş.';
		}
	});

	async function download() {
		downloading = true;
		try {
			const blob = await shareApi.download(token!);
			const url = URL.createObjectURL(blob);
			const a = document.createElement('a');
			a.href = url;
			a.download = info.name;
			a.click();
			URL.revokeObjectURL(url);
		} catch (e: any) {
			error = 'İndirme başarısız.';
		} finally {
			downloading = false;
		}
	}
</script>

<div class="share-page">
	<div class="share-card card">
		<h2>Paylaşılan Dosya</h2>

		{#if error}
			<div class="error">{error}</div>
		{:else if !info}
			<p class="loading">Yükleniyor...</p>
		{:else}
			<div class="file-info">
				<div class="file-icon">{info.type === 'folder' ? '📁' : '📄'}</div>
				<div>
					<p class="file-name">{info.name}</p>
					{#if info.size}
						<p class="file-size">{formatBytes(info.size)}</p>
					{/if}
				</div>
			</div>

			{#if info.type === 'file'}
				<button class="btn-primary download-btn" onclick={download} disabled={downloading}>
					{downloading ? 'İndiriliyor...' : 'İndir'}
				</button>
			{/if}
		{/if}
	</div>
</div>

<style>
	.share-page {
		display: flex;
		align-items: center;
		justify-content: center;
		min-height: 100vh;
		padding: 1rem;
	}

	.share-card {
		width: 100%;
		max-width: 420px;
		text-align: center;
	}

	h2 {
		margin-bottom: 1.5rem;
		color: var(--primary);
	}

	.error {
		background: rgba(239, 68, 68, 0.1);
		border: 1px solid var(--danger);
		color: var(--danger);
		padding: 0.75rem;
		border-radius: var(--radius);
		font-size: 0.875rem;
	}

	.loading {
		color: var(--text-muted);
	}

	.file-info {
		display: flex;
		align-items: center;
		gap: 1rem;
		padding: 1rem;
		background: var(--bg-tertiary);
		border-radius: var(--radius);
		margin-bottom: 1.5rem;
		text-align: left;
	}

	.file-icon {
		font-size: 2.5rem;
	}

	.file-name {
		font-weight: 600;
		font-size: 1rem;
	}

	.file-size {
		color: var(--text-muted);
		font-size: 0.8125rem;
		margin-top: 0.125rem;
	}

	.download-btn {
		width: 100%;
		padding: 0.75rem;
		font-size: 1rem;
	}
</style>
