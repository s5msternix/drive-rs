<script>
	import { auth as authApi } from '$lib/api/client';
	import { setAuth } from '$lib/stores/auth';
	import { goto } from '$app/navigation';

	let email = $state('');
	let username = $state('');
	let password = $state('');
	let error = $state('');
	let loading = $state(false);

	async function handleRegister() {
		error = '';
		if (password.length < 8) {
			error = 'Şifre en az 8 karakter olmalı';
			return;
		}
		loading = true;
		try {
			const res = await authApi.register({ email, username, password });
			setAuth(res.token, res.user);
			goto('/drive');
		} catch (e) {
			error = 'Kayıt başarısız. E-posta veya kullanıcı adı zaten kullanılıyor olabilir.';
		} finally {
			loading = false;
		}
	}
</script>

<div class="auth-page">
	<div class="auth-card card">
		<h1>Drive-RS</h1>
		<p class="subtitle">Yeni hesap oluşturun</p>

		{#if error}
			<div class="error">{error}</div>
		{/if}

		<form onsubmit={handleRegister}>
			<div class="field">
				<label for="username">Kullanıcı Adı</label>
				<input id="username" type="text" bind:value={username} placeholder="kullaniciadi" required />
			</div>
			<div class="field">
				<label for="email">E-posta</label>
				<input id="email" type="email" bind:value={email} placeholder="ornek@email.com" required />
			</div>
			<div class="field">
				<label for="password">Şifre</label>
				<input id="password" type="password" bind:value={password} placeholder="En az 8 karakter" required />
			</div>
			<button type="submit" class="btn-primary submit" disabled={loading}>
				{loading ? 'Kayıt yapılıyor...' : 'Kayıt Ol'}
			</button>
		</form>

		<p class="footer">Zaten hesabınız var mı? <a href="/login">Giriş Yap</a></p>
	</div>
</div>

<style>
	.auth-page {
		display: flex;
		align-items: center;
		justify-content: center;
		min-height: 100vh;
		padding: 1rem;
	}

	.auth-card {
		width: 100%;
		max-width: 400px;
	}

	h1 {
		font-size: 1.75rem;
		text-align: center;
		color: var(--primary);
		margin-bottom: 0.25rem;
	}

	.subtitle {
		text-align: center;
		color: var(--text-muted);
		margin-bottom: 1.5rem;
		font-size: 0.875rem;
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

	form {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.field {
		display: flex;
		flex-direction: column;
		gap: 0.375rem;
	}

	label {
		font-size: 0.8125rem;
		color: var(--text-muted);
	}

	.submit {
		width: 100%;
		padding: 0.625rem;
		margin-top: 0.5rem;
	}

	.footer {
		text-align: center;
		margin-top: 1.25rem;
		font-size: 0.8125rem;
		color: var(--text-muted);
	}
</style>
