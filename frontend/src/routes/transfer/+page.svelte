<script lang="ts">
	import { transfer as transferApi } from '$lib/api/client';
	import { formatBytes } from '$lib/stores/auth';

	type Mode = 'idle' | 'sending' | 'receiving';

	let mode: Mode = $state('idle');
	let selectedFile: File | null = $state(null);
	let sessionToken = $state('');
	let receiveToken = $state('');
	let transferUrl = $state('');
	let status = $state('');
	let progress = $state(0);
	let error = $state('');
	let ws: WebSocket | null = $state(null);
	let pc: RTCPeerConnection | null = $state(null);

	const rtcConfig: RTCConfiguration = {
		iceServers: [{ urls: 'stun:stun.l.google.com:19302' }]
	};

	// ---- SENDER ----
	async function startSending() {
		if (!selectedFile) return;
		error = '';
		mode = 'sending';
		status = 'Oturum oluşturuluyor...';

		try {
			const session = await transferApi.createSession({
				file_name: selectedFile.name,
				file_size: selectedFile.size
			});
			sessionToken = session.token;
			transferUrl = `${window.location.origin}/transfer?receive=${session.token}`;
			status = 'Alıcı bekleniyor... Linki paylaşın.';

			// Connect signaling
			ws = transferApi.connectSignaling(session.token);
			ws.onmessage = async (e) => {
				const msg = JSON.parse(e.data);
				if (msg.type === 'peer-joined') {
					await createOffer();
				} else if (msg.type === 'answer' && pc) {
					await pc.setRemoteDescription({ type: 'answer', sdp: msg.sdp });
				} else if (msg.type === 'ice-candidate' && pc) {
					try {
						await pc.addIceCandidate(JSON.parse(msg.candidate));
					} catch {}
				}
			};
		} catch (e: any) {
			error = 'Oturum oluşturulamadı';
			mode = 'idle';
		}
	}

	async function createOffer() {
		if (!selectedFile || !ws) return;
		status = 'Bağlantı kuruluyor...';

		pc = new RTCPeerConnection(rtcConfig);
		const channel = pc.createDataChannel('file-transfer');

		pc.onicecandidate = (e) => {
			if (e.candidate && ws) {
				ws.send(JSON.stringify({ type: 'ice-candidate', candidate: JSON.stringify(e.candidate) }));
			}
		};

		channel.onopen = () => {
			status = 'Dosya gönderiliyor...';
			sendFileOverChannel(channel);
		};

		const offer = await pc.createOffer();
		await pc.setLocalDescription(offer);
		ws.send(JSON.stringify({ type: 'offer', sdp: offer.sdp }));
	}

	async function sendFileOverChannel(channel: RTCDataChannel) {
		if (!selectedFile) return;
		const CHUNK_SIZE = 64 * 1024; // 64KB
		const file = selectedFile;
		let offset = 0;

		// Send metadata first
		channel.send(JSON.stringify({ name: file.name, size: file.size, type: file.type }));

		const reader = new FileReader();

		function readSlice() {
			const slice = file.slice(offset, offset + CHUNK_SIZE);
			reader.readAsArrayBuffer(slice);
		}

		reader.onload = () => {
			if (!reader.result) return;
			channel.send(reader.result as ArrayBuffer);
			offset += (reader.result as ArrayBuffer).byteLength;
			progress = Math.min(100, (offset / file.size) * 100);

			if (offset < file.size) {
				// Back-pressure: wait if buffered amount is too high
				if (channel.bufferedAmount > CHUNK_SIZE * 8) {
					setTimeout(readSlice, 50);
				} else {
					readSlice();
				}
			} else {
				status = 'Transfer tamamlandı!';
				channel.send('__DONE__');
			}
		};

		readSlice();
	}

	// ---- RECEIVER ----
	async function startReceiving() {
		if (!receiveToken.trim()) return;
		error = '';
		mode = 'receiving';
		status = 'Oturuma bağlanılıyor...';

		try {
			const session = await transferApi.getSession(receiveToken.trim());
			status = `"${session.file_name}" (${formatBytes(session.file_size)}) dosyası alınacak. Gönderici bekleniyor...`;

			ws = transferApi.connectSignaling(receiveToken.trim());
			ws.onmessage = async (e) => {
				const msg = JSON.parse(e.data);
				if (msg.type === 'offer') {
					await handleOffer(msg.sdp);
				} else if (msg.type === 'ice-candidate' && pc) {
					try {
						await pc.addIceCandidate(JSON.parse(msg.candidate));
					} catch {}
				}
			};
		} catch (e: any) {
			error = 'Oturum bulunamadı veya süresi dolmuş';
			mode = 'idle';
		}
	}

	async function handleOffer(sdp: string) {
		if (!ws) return;
		status = 'Bağlantı kuruluyor...';

		pc = new RTCPeerConnection(rtcConfig);

		pc.onicecandidate = (e) => {
			if (e.candidate && ws) {
				ws.send(JSON.stringify({ type: 'ice-candidate', candidate: JSON.stringify(e.candidate) }));
			}
		};

		let receivedChunks: ArrayBuffer[] = [];
		let fileInfo: { name: string; size: number; type: string } | null = null;

		pc.ondatachannel = (e) => {
			const channel = e.channel;
			channel.binaryType = 'arraybuffer';

			channel.onmessage = (ev) => {
				if (typeof ev.data === 'string') {
					if (ev.data === '__DONE__') {
						// Assemble and download
						const blob = new Blob(receivedChunks, { type: fileInfo?.type || 'application/octet-stream' });
						const url = URL.createObjectURL(blob);
						const a = document.createElement('a');
						a.href = url;
						a.download = fileInfo?.name || 'download';
						a.click();
						URL.revokeObjectURL(url);
						status = 'Transfer tamamlandı! Dosya indirildi.';
						return;
					}
					// File metadata
					try {
						fileInfo = JSON.parse(ev.data);
						status = `"${fileInfo!.name}" alınıyor...`;
					} catch {}
					return;
				}

				receivedChunks.push(ev.data);
				if (fileInfo) {
					const received = receivedChunks.reduce((sum, c) => sum + c.byteLength, 0);
					progress = Math.min(100, (received / fileInfo.size) * 100);
				}
			};
		};

		await pc.setRemoteDescription({ type: 'offer', sdp });
		const answer = await pc.createAnswer();
		await pc.setLocalDescription(answer);
		ws.send(JSON.stringify({ type: 'answer', sdp: answer.sdp }));
	}

	function reset() {
		ws?.close();
		pc?.close();
		ws = null;
		pc = null;
		mode = 'idle';
		selectedFile = null;
		sessionToken = '';
		receiveToken = '';
		transferUrl = '';
		status = '';
		progress = 0;
		error = '';
	}

	// Auto-fill receive token from URL params
	import { onMount } from 'svelte';
	import { page } from '$app/stores';

	onMount(() => {
		const receive = $page.url.searchParams.get('receive');
		if (receive) {
			receiveToken = receive;
			startReceiving();
		}
	});

	let fileInput: HTMLInputElement;
</script>

<div class="transfer-page">
	<div class="transfer-container">
		<h1>Doğrudan Dosya Transferi</h1>
		<p class="subtitle">Dosyalarınızı sunucuda depolamadan, doğrudan cihazdan cihaza aktarın</p>

		{#if error}
			<div class="error">{error}</div>
		{/if}

		{#if mode === 'idle'}
			<div class="options">
				<!-- Send -->
				<div class="option-card card">
					<h2>Dosya Gönder</h2>
					<p>Cihazınızdan doğrudan bir dosya gönderin</p>

					{#if selectedFile}
						<div class="selected-file">
							<span>📄 {selectedFile.name}</span>
							<span class="file-size">{formatBytes(selectedFile.size)}</span>
						</div>
					{/if}

					<div class="option-actions">
						<button class="btn-ghost" onclick={() => fileInput.click()}>
							{selectedFile ? 'Değiştir' : 'Dosya Seç'}
						</button>
						<input
							type="file"
							bind:this={fileInput}
							onchange={(e) => {
								const f = (e.target as HTMLInputElement).files;
								if (f && f.length > 0) selectedFile = f[0];
							}}
							hidden
						/>
						{#if selectedFile}
							<button class="btn-primary" onclick={startSending}>Gönder</button>
						{/if}
					</div>
				</div>

				<!-- Receive -->
				<div class="option-card card">
					<h2>Dosya Al</h2>
					<p>Transfer kodunu girerek dosya alın</p>
					<input
						type="text"
						bind:value={receiveToken}
						placeholder="Transfer kodu yapıştırın"
					/>
					<div class="option-actions">
						<button class="btn-primary" onclick={startReceiving} disabled={!receiveToken.trim()}>
							Bağlan
						</button>
					</div>
				</div>
			</div>
		{:else}
			<!-- Active transfer -->
			<div class="active-transfer card">
				<p class="status">{status}</p>

				{#if transferUrl}
					<div class="share-link">
						<label>Bu linki alıcıyla paylaşın:</label>
						<div class="link-row">
							<input type="text" value={transferUrl} readonly />
							<button class="btn-primary" onclick={() => navigator.clipboard.writeText(transferUrl)}>
								Kopyala
							</button>
						</div>
					</div>
				{/if}

				{#if progress > 0}
					<div class="progress-section">
						<div class="progress-bg">
							<div class="progress-fill" style="width: {progress}%"></div>
						</div>
						<span class="progress-text">{progress.toFixed(1)}%</span>
					</div>
				{/if}

				<button class="btn-ghost" onclick={reset}>
					{status.includes('tamamlandı') ? 'Yeni Transfer' : 'İptal'}
				</button>
			</div>
		{/if}
	</div>
</div>

<style>
	.transfer-page {
		display: flex;
		justify-content: center;
		padding: 2rem 1rem;
	}

	.transfer-container {
		max-width: 700px;
		width: 100%;
	}

	h1 {
		font-size: 1.5rem;
		text-align: center;
		margin-bottom: 0.25rem;
	}

	.subtitle {
		text-align: center;
		color: var(--text-muted);
		font-size: 0.875rem;
		margin-bottom: 2rem;
	}

	.error {
		background: rgba(239, 68, 68, 0.1);
		border: 1px solid var(--danger);
		color: var(--danger);
		padding: 0.5rem 0.75rem;
		border-radius: var(--radius);
		font-size: 0.8125rem;
		margin-bottom: 1rem;
	}

	.options {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 1rem;
	}

	@media (max-width: 600px) {
		.options {
			grid-template-columns: 1fr;
		}
	}

	.option-card {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.option-card h2 {
		font-size: 1.125rem;
	}

	.option-card p {
		color: var(--text-muted);
		font-size: 0.8125rem;
	}

	.selected-file {
		display: flex;
		justify-content: space-between;
		align-items: center;
		background: var(--bg-tertiary);
		padding: 0.5rem 0.75rem;
		border-radius: var(--radius);
		font-size: 0.8125rem;
	}

	.file-size {
		color: var(--text-muted);
	}

	.option-actions {
		display: flex;
		gap: 0.5rem;
		margin-top: auto;
	}

	.active-transfer {
		display: flex;
		flex-direction: column;
		gap: 1rem;
		align-items: center;
		text-align: center;
	}

	.status {
		font-size: 1rem;
		font-weight: 500;
	}

	.share-link {
		width: 100%;
		text-align: left;
	}

	.share-link label {
		display: block;
		font-size: 0.8125rem;
		color: var(--text-muted);
		margin-bottom: 0.375rem;
	}

	.link-row {
		display: flex;
		gap: 0.5rem;
	}

	.link-row input {
		flex: 1;
	}

	.progress-section {
		width: 100%;
		display: flex;
		align-items: center;
		gap: 0.75rem;
	}

	.progress-bg {
		flex: 1;
		background: var(--bg-tertiary);
		border-radius: 4px;
		height: 8px;
		overflow: hidden;
	}

	.progress-fill {
		background: var(--primary);
		height: 100%;
		border-radius: 4px;
		transition: width 0.2s;
	}

	.progress-text {
		font-size: 0.875rem;
		color: var(--text-muted);
		min-width: 50px;
		text-align: right;
	}
</style>
