<script lang="ts">
	import { onMount } from 'svelte';
	import { files as filesApi, folders as foldersApi, share as shareApi } from '$lib/api/client';
	import { user } from '$lib/stores/auth';
	import { formatBytes } from '$lib/stores/auth';

	let currentFolderId: string | undefined = $state(undefined);
	let breadcrumbs: { id?: string; name: string }[] = $state([{ name: 'Ana Dizin' }]);
	let folderList: any[] = $state([]);
	let fileList: any[] = $state([]);
	let loading = $state(true);
	let showNewFolder = $state(false);
	let newFolderName = $state('');
	let dragOver = $state(false);
	let uploading = $state(false);
	let shareModal: { show: boolean; url: string } = $state({ show: false, url: '' });

	onMount(() => loadContents());

	async function loadContents() {
		loading = true;
		try {
			const [f, d] = await Promise.all([
				filesApi.list(currentFolderId),
				foldersApi.list(currentFolderId)
			]);
			fileList = f;
			folderList = d;
		} catch (e) {
			console.error(e);
		} finally {
			loading = false;
		}
	}

	async function navigateToFolder(folderId: string | undefined, folderName?: string) {
		if (folderId === currentFolderId) return;

		if (!folderId) {
			breadcrumbs = [{ name: 'Ana Dizin' }];
		} else if (folderName) {
			const idx = breadcrumbs.findIndex((b) => b.id === folderId);
			if (idx >= 0) {
				breadcrumbs = breadcrumbs.slice(0, idx + 1);
			} else {
				breadcrumbs = [...breadcrumbs, { id: folderId, name: folderName }];
			}
		}

		currentFolderId = folderId;
		await loadContents();
	}

	async function createFolder() {
		if (!newFolderName.trim()) return;
		try {
			await foldersApi.create(newFolderName.trim(), currentFolderId);
			newFolderName = '';
			showNewFolder = false;
			await loadContents();
		} catch (e) {
			console.error(e);
		}
	}

	async function deleteFolder(id: string) {
		if (!confirm('Bu klasörü silmek istediğinize emin misiniz?')) return;
		try {
			await foldersApi.delete(id);
			await loadContents();
		} catch (e) {
			console.error(e);
		}
	}

	async function deleteFile(id: string) {
		if (!confirm('Bu dosyayı silmek istediğinize emin misiniz?')) return;
		try {
			await filesApi.delete(id);
			await loadContents();
		} catch (e) {
			console.error(e);
		}
	}

	async function downloadFile(id: string, name: string) {
		try {
			const blob = await filesApi.download(id);
			const url = URL.createObjectURL(blob);
			const a = document.createElement('a');
			a.href = url;
			a.download = name;
			a.click();
			URL.revokeObjectURL(url);
		} catch (e) {
			console.error(e);
		}
	}

	async function handleUpload(filesList: FileList | null) {
		if (!filesList || filesList.length === 0) return;
		uploading = true;
		try {
			for (const file of filesList) {
				await filesApi.upload(file, currentFolderId);
			}
			await loadContents();
		} catch (e) {
			console.error(e);
		} finally {
			uploading = false;
		}
	}

	function handleDrop(e: DragEvent) {
		e.preventDefault();
		dragOver = false;
		handleUpload(e.dataTransfer?.files ?? null);
	}

	function handleDragOver(e: DragEvent) {
		e.preventDefault();
		dragOver = true;
	}

	async function shareFile(fileId: string) {
		try {
			const res = await shareApi.createLink({ file_id: fileId, expires_in_hours: 72 });
			shareModal = { show: true, url: `${window.location.origin}/share/${res.token}` };
		} catch (e) {
			console.error(e);
		}
	}

	function getFileIcon(mime: string): string {
		if (mime.startsWith('image/')) return '🖼';
		if (mime.startsWith('video/')) return '🎬';
		if (mime.startsWith('audio/')) return '🎵';
		if (mime.includes('pdf')) return '📄';
		if (mime.includes('zip') || mime.includes('tar') || mime.includes('rar')) return '📦';
		if (mime.includes('text') || mime.includes('json') || mime.includes('xml')) return '📝';
		return '📎';
	}

	let fileInput: HTMLInputElement;
</script>

<div
	class="drive-page"
	ondrop={handleDrop}
	ondragover={handleDragOver}
	ondragleave={() => (dragOver = false)}
	role="main"
>
	<!-- Header -->
	<div class="drive-header">
		<div class="breadcrumbs">
			{#each breadcrumbs as crumb, i}
				{#if i > 0}<span class="sep">/</span>{/if}
				<button
					class="crumb"
					class:active={i === breadcrumbs.length - 1}
					onclick={() => navigateToFolder(crumb.id, crumb.name)}
				>
					{crumb.name}
				</button>
			{/each}
		</div>
		<div class="actions">
			<button class="btn-ghost" onclick={() => (showNewFolder = !showNewFolder)}>Yeni Klasör</button>
			<button class="btn-primary" onclick={() => fileInput.click()}>
				Dosya Yükle
			</button>
			<input
				type="file"
				multiple
				bind:this={fileInput}
				onchange={(e) => handleUpload((e.target as HTMLInputElement).files)}
				hidden
			/>
		</div>
	</div>

	<!-- Storage info -->
	{#if $user}
		<div class="storage-bar">
			<div class="storage-info">
				<span>{formatBytes($user.storage_used)} / {formatBytes($user.storage_limit)} kullanıldı</span>
			</div>
			<div class="progress-bg">
				<div
					class="progress-fill"
					style="width: {($user.storage_used / $user.storage_limit) * 100}%"
				></div>
			</div>
		</div>
	{/if}

	<!-- New folder input -->
	{#if showNewFolder}
		<div class="new-folder-row">
			<input
				type="text"
				bind:value={newFolderName}
				placeholder="Klasör adı"
				onkeydown={(e) => e.key === 'Enter' && createFolder()}
			/>
			<button class="btn-primary" onclick={createFolder}>Oluştur</button>
			<button class="btn-ghost" onclick={() => (showNewFolder = false)}>İptal</button>
		</div>
	{/if}

	<!-- Drag overlay -->
	{#if dragOver}
		<div class="drop-overlay">
			<p>Dosyaları buraya bırakın</p>
		</div>
	{/if}

	<!-- Upload indicator -->
	{#if uploading}
		<div class="uploading-bar">Yükleniyor...</div>
	{/if}

	<!-- Content -->
	{#if loading}
		<div class="empty-state">Yükleniyor...</div>
	{:else if folderList.length === 0 && fileList.length === 0}
		<div class="empty-state">
			<p>Bu klasör boş</p>
			<p class="hint">Dosya yükleyin veya yeni bir klasör oluşturun</p>
		</div>
	{:else}
		<div class="file-grid">
			{#each folderList as folder}
				<div class="file-item folder" ondblclick={() => navigateToFolder(folder.id, folder.name)} role="button" tabindex="0">
					<div class="file-icon">📁</div>
					<div class="file-info">
						<span class="file-name">{folder.name}</span>
						<span class="file-meta">Klasör</span>
					</div>
					<div class="file-actions">
						<button class="btn-icon" title="Sil" onclick={() => deleteFolder(folder.id)}>🗑</button>
					</div>
				</div>
			{/each}

			{#each fileList as file}
				<div class="file-item" role="button" tabindex="0">
					<div class="file-icon">{getFileIcon(file.mime_type)}</div>
					<div class="file-info">
						<span class="file-name">{file.name}</span>
						<span class="file-meta">{formatBytes(file.size)}</span>
					</div>
					<div class="file-actions">
						<button class="btn-icon" title="İndir" onclick={() => downloadFile(file.id, file.original_name)}>⬇</button>
						<button class="btn-icon" title="Paylaş" onclick={() => shareFile(file.id)}>🔗</button>
						<button class="btn-icon" title="Sil" onclick={() => deleteFile(file.id)}>🗑</button>
					</div>
				</div>
			{/each}
		</div>
	{/if}

	<!-- Share Modal -->
	{#if shareModal.show}
		<div class="modal-overlay" onclick={() => (shareModal.show = false)} role="dialog">
			<div class="modal card" onclick={(e) => e.stopPropagation()} role="document">
				<h3>Paylaşım Linki</h3>
				<input type="text" value={shareModal.url} readonly />
				<div class="modal-actions">
					<button
						class="btn-primary"
						onclick={() => {
							navigator.clipboard.writeText(shareModal.url);
						}}
					>
						Kopyala
					</button>
					<button class="btn-ghost" onclick={() => (shareModal.show = false)}>Kapat</button>
				</div>
			</div>
		</div>
	{/if}
</div>

<style>
	.drive-page {
		max-width: 1000px;
		margin: 0 auto;
		padding: 1.5rem;
		position: relative;
	}

	.drive-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 1rem;
		flex-wrap: wrap;
		gap: 0.75rem;
	}

	.breadcrumbs {
		display: flex;
		align-items: center;
		gap: 0.25rem;
		flex-wrap: wrap;
	}

	.crumb {
		background: none;
		color: var(--text-muted);
		padding: 0.25rem 0.5rem;
		font-size: 0.875rem;
	}

	.crumb.active {
		color: var(--text);
		font-weight: 600;
	}

	.crumb:hover {
		color: var(--primary);
	}

	.sep {
		color: var(--text-muted);
		font-size: 0.75rem;
	}

	.actions {
		display: flex;
		gap: 0.5rem;
	}

	.storage-bar {
		margin-bottom: 1rem;
	}

	.storage-info {
		font-size: 0.75rem;
		color: var(--text-muted);
		margin-bottom: 0.25rem;
	}

	.progress-bg {
		background: var(--bg-tertiary);
		border-radius: 4px;
		height: 4px;
		overflow: hidden;
	}

	.progress-fill {
		background: var(--primary);
		height: 100%;
		border-radius: 4px;
		transition: width 0.3s;
	}

	.new-folder-row {
		display: flex;
		gap: 0.5rem;
		margin-bottom: 1rem;
	}

	.new-folder-row input {
		max-width: 300px;
	}

	.drop-overlay {
		position: fixed;
		inset: 0;
		background: rgba(99, 102, 241, 0.1);
		border: 3px dashed var(--primary);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 50;
		font-size: 1.25rem;
		color: var(--primary);
	}

	.uploading-bar {
		background: var(--primary);
		color: white;
		padding: 0.5rem 1rem;
		border-radius: var(--radius);
		margin-bottom: 1rem;
		font-size: 0.875rem;
		text-align: center;
	}

	.empty-state {
		text-align: center;
		padding: 4rem 1rem;
		color: var(--text-muted);
	}

	.hint {
		font-size: 0.8125rem;
		margin-top: 0.5rem;
	}

	.file-grid {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.file-item {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 0.75rem 1rem;
		border-radius: var(--radius);
		background: var(--bg-secondary);
		transition: background 0.1s;
		cursor: default;
	}

	.file-item:hover {
		background: var(--bg-tertiary);
	}

	.file-item.folder {
		cursor: pointer;
	}

	.file-icon {
		font-size: 1.5rem;
		width: 2rem;
		text-align: center;
		flex-shrink: 0;
	}

	.file-info {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
	}

	.file-name {
		font-size: 0.875rem;
		font-weight: 500;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.file-meta {
		font-size: 0.75rem;
		color: var(--text-muted);
	}

	.file-actions {
		display: flex;
		gap: 0.25rem;
		opacity: 0;
		transition: opacity 0.15s;
	}

	.file-item:hover .file-actions {
		opacity: 1;
	}

	.btn-icon {
		background: none;
		padding: 0.375rem;
		font-size: 1rem;
		border-radius: var(--radius);
		line-height: 1;
	}

	.btn-icon:hover {
		background: var(--bg-tertiary);
	}

	.modal-overlay {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.6);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 200;
	}

	.modal {
		width: 100%;
		max-width: 450px;
	}

	.modal h3 {
		margin-bottom: 1rem;
	}

	.modal input {
		margin-bottom: 1rem;
	}

	.modal-actions {
		display: flex;
		gap: 0.5rem;
		justify-content: flex-end;
	}
</style>
