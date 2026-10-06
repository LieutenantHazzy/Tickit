function app() {
  return {
    lang: 'en',
    view: 'todos',
    user: null,
    todos: [],
    authMode: 'login',
    flash: '',
    login: { email: '', password: '', totp: '' },
    totp: { secret: '', code: '' },
    settings: { lang: 'en', hour: 7, min: 0 },
    newTodo: { title: '', due_date: '', due_time: '' },
    translations: {},
    easymde: null,

    t(key) {
      return (this.translations[this.lang] && this.translations[this.lang][key]) || key;
    },

    async init() {
      this.lang = localStorage.getItem('tickit_lang') || 'en';
      await this.loadTranslations();
      await this.checkAuth();
      if (this.user) {
        await this.loadTodos();
        this.$nextTick(() => this.initEditor());
      }
    },

    async initEditor() {
      const el = document.getElementById('md-editor');
      if (el && !this.easymde) {
        this.easymde = new EasyMDE({ element: el, spellChecker: false, status: false });
      }
    },

    async loadTranslations() {
      try {
        const r = await fetch('/assets/js/i18n/' + this.lang + '.json');
        this.translations[this.lang] = await r.json();
      } catch (e) {}
    },

    toggleLang() {
      this.lang = this.lang === 'en' ? 'nl' : 'en';
      localStorage.setItem('tickit_lang', this.lang);
      this.loadTranslations();
    },

    toggleAuthMode() {
      this.authMode = this.authMode === 'login' ? 'register' : 'login';
      this.flash = '';
    },

    async checkAuth() {
      try {
        const r = await fetch('/api/auth/me');
        if (r.ok) {
          this.user = await r.json();
          this.lang = this.user.lang;
          this.settings.lang = this.user.lang;
          this.settings.hour = this.user.daily_reminder_hour;
          this.settings.min = this.user.daily_reminder_min;
          localStorage.setItem('tickit_lang', this.lang);
        }
      } catch (e) {}
    },

    async doLogin() {
      this.flash = '';
      const url = this.authMode === 'login' ? '/api/auth/login' : '/api/auth/register';
      try {
        const r = await fetch(url, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            email: this.login.email,
            password: this.login.password,
            totp_code: this.login.totp || null
          })
        });
        if (r.ok) {
          this.login = { email: '', password: '', totp: '' };
          await this.checkAuth();
          if (this.user) {
            await this.loadTodos();
            this.$nextTick(() => this.initEditor());
          }
        } else {
          const data = await r.json().catch(() => ({}));
          this.flash = data.error || 'failed';
        }
      } catch (e) {
        this.flash = 'network error';
      }
    },

    async logout() {
      try { await fetch('/api/auth/logout', { method: 'POST' }); } catch (e) {}
      this.user = null;
      this.todos = [];
      this.authMode = 'login';
    },

    showTodos() { this.view = 'todos'; this.$nextTick(() => this.initEditor()); },
    showSettings() { this.view = 'settings'; },

    async loadTodos() {
      const r = await fetch('/api/todos');
      if (r.ok) this.todos = await r.json();
    },

    async addTodo() {
      const desc = this.easymde ? this.easymde.value() : '';
      await fetch('/api/todos', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          title: this.newTodo.title,
          description_md: desc || null,
          due_date: this.newTodo.due_date || null,
          due_time: this.newTodo.due_time || null
        })
      });
      this.newTodo = { title: '', due_date: '', due_time: '' };
      if (this.easymde) this.easymde.value('');
      await this.loadTodos();
    },

    async toggleTodo(id) {
      await fetch('/api/todos/' + id + '/toggle', { method: 'POST' });
      await this.loadTodos();
    },

    renderMarkdown(md) {
      if (!md) return '';
      try {
        if (window.marked) return marked.parse(md);
      } catch (e) {}
      const escape = md.replace(/</g, '&lt;').replace(/>/g, '&gt;');
      return '<p class="whitespace-pre-wrap">' + escape + '</p>';
    },

    formatDue(todo) {
      let s = '';
      if (todo.due_date) s += todo.due_date;
      if (todo.due_time) s += ' ' + todo.due_time;
      return s;
    },

    async saveSettings() {
      await fetch('/api/auth/preferences', {
        method: 'PATCH',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          lang: this.settings.lang,
          daily_reminder_hour: this.settings.hour,
          daily_reminder_min: this.settings.min
        })
      });
      this.lang = this.settings.lang;
      localStorage.setItem('tickit_lang', this.lang);
      this.loadTranslations();
    },

    async setupTotp() {
      const r = await fetch('/api/auth/setup-2fa');
      if (r.ok) {
        this.totp = await r.json();
        await this.$nextTick();
        const el = document.getElementById('qrcode');
        if (el && window.QRCode) {
          el.innerHTML = '';
          new QRCode(el, { text: this.totp.qr_url, width: 180, height: 180 });
        }
      }
    },

    async enableTotp() {
      const r = await fetch('/api/auth/setup-2fa', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ code: this.totp.code })
      });
      if (r.ok) {
        this.totp = { secret: '', code: '' };
        await this.checkAuth();
      }
    },

    async disableTotp() {
      await fetch('/api/auth/disable-2fa', { method: 'POST' });
      await this.checkAuth();
    },

    async exportJson() {
      const r = await fetch('/api/todos/export/json');
      if (r.ok) {
        const data = await r.json();
        const blob = new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' });
        const a = document.createElement('a');
        a.href = URL.createObjectURL(blob);
        a.download = 'tickit-export.json';
        a.click();
        URL.revokeObjectURL(a.href);
      }
    }
  };
}
