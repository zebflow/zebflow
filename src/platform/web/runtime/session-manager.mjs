// Session manager for MCP remote control

(function () {
  function initSessionPanel() {
    const panel = document.querySelector('.project-shell-session');
    if (!panel) return;

    const owner = panel.dataset.owner;
    const project = panel.dataset.project;
    if (!owner || !project) return;

    const toggle = panel.querySelector('.project-shell-session-toggle');
    const tokenInput = panel.querySelector('.project-shell-session-token-input');
    const urlInput = panel.querySelector('.project-shell-session-url-input');
    const copyBtn = panel.querySelector('.project-shell-session-copy-button');
    const operationChecks = panel.querySelectorAll('.project-shell-session-operations input[type="checkbox"]');

    const apiBase = `/api/projects/${owner}/${project}/mcp/session`;

    async function loadSession() {
      try {
        const resp = await fetch(apiBase);
        if (resp.ok) {
          const data = await resp.json();
          if (data.ok && data.session) {
            toggle.checked = true;
            // Note: we don't get the token back on GET, only on POST
          }
        }
      } catch (err) {
        console.error('Failed to load session:', err);
      }
    }

    async function createSession() {
      const capabilities = Array.from(operationChecks)
        .filter(c => c.checked)
        .map(c => c.value);

      try {
        const resp = await fetch(apiBase, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ capabilities }),
        });
        const data = await resp.json();
        if (data.ok && data.session) {
          tokenInput.value = data.session.token;
          urlInput.value = data.session.mcp_url;
        } else {
          alert('Failed to create session');
        }
      } catch (err) {
        console.error('Failed to create session:', err);
        alert('Failed to create session');
      }
    }

    async function revokeSession() {
      try {
        await fetch(apiBase, { method: 'DELETE' });
        tokenInput.value = '';
        urlInput.value = '';
      } catch (err) {
        console.error('Failed to revoke session:', err);
      }
    }

    toggle.addEventListener('change', async () => {
      if (toggle.checked) {
        await createSession();
      } else {
        await revokeSession();
      }
    });

    copyBtn.addEventListener('click', () => {
      tokenInput.select();
      document.execCommand('copy');
      copyBtn.textContent = 'Copied!';
      setTimeout(() => {
        copyBtn.textContent = 'Copy';
      }, 2000);
    });

    loadSession();
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', initSessionPanel);
  } else {
    initSessionPanel();
  }
})();
