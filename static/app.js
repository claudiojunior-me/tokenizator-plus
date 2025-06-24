document.addEventListener('DOMContentLoaded', () => {
    // Referências aos elementos do DOM
    const pathForm = document.getElementById('path-form');
    const pathInput = document.getElementById('path-input');
    const output = document.getElementById('output');
    const copyButton = document.getElementById('copy-button');
    const tokenCountSpan = document.getElementById('token-count');
    const controls = document.getElementById('controls');
    const loadingIndicator = document.getElementById('loading');
    const cancelButton = document.getElementById('cancel-button');
    const themeToggle = document.getElementById('theme-toggle');
    
    // Novos elementos para a feature de ignorar
    const ignoreForm = document.getElementById('ignore-form');
    const ignoreInput = document.getElementById('ignore-input');
    const ignoreList = document.getElementById('ignore-list');

    // Estado da aplicação
    const defaultIgnorePatterns = [
        '.git',
        'node_modules',
        'target',
        'pnpm-lock.yaml',
        'yarn.lock',
        'package-lock.json',
        '.env'
    ];
    let ignorePatterns = [...defaultIgnorePatterns];
    let currentController = null;

    // --- TEMA CLARO/ESCURO ---
    const setTheme = (theme) => {
        document.documentElement.setAttribute('data-theme', theme);
        localStorage.setItem('theme', theme);
        themeToggle.textContent = theme === 'dark' ? 'Tema Claro' : 'Tema Escuro';
    };

    const savedTheme = localStorage.getItem('theme') || 'dark';
    setTheme(savedTheme);

    themeToggle.addEventListener('click', () => {
        const current = document.documentElement.getAttribute('data-theme');
        const next = current === 'dark' ? 'light' : 'dark';
        setTheme(next);
    });

    // --- LÓGICA PARA GERENCIAR A LISTA DE PADRÕES ---

    const renderIgnoreList = () => {
        ignoreList.innerHTML = ''; // Limpa a lista atual
        ignorePatterns.forEach(pattern => {
            const li = document.createElement('li');
            li.className = 'tag-item';
            li.textContent = pattern;

            const deleteBtn = document.createElement('button');
            deleteBtn.className = 'delete-tag-btn';
            deleteBtn.textContent = '×'; // Símbolo de "X" para fechar
            deleteBtn.dataset.pattern = pattern; // Guarda o padrão no botão
            
            li.appendChild(deleteBtn);
            ignoreList.appendChild(li);
        });
    };

    // Renderiza a lista inicial com padrões pré-definidos
    renderIgnoreList();

    ignoreForm.addEventListener('submit', (e) => {
        e.preventDefault();
        const newPattern = ignoreInput.value.trim();
        if (newPattern && !ignorePatterns.includes(newPattern)) {
            ignorePatterns.push(newPattern);
            renderIgnoreList();
        }
        ignoreInput.value = ''; // Limpa o input
    });

    ignoreList.addEventListener('click', (e) => {
        if (e.target.classList.contains('delete-tag-btn')) {
            const patternToRemove = e.target.dataset.pattern;
            ignorePatterns = ignorePatterns.filter(p => p !== patternToRemove);
            renderIgnoreList();
        }
    });


    // --- LÓGICA PRINCIPAL (ANÁLISE DE PASTA) ---

    pathForm.addEventListener('submit', async (e) => {
        e.preventDefault();
        const path = pathInput.value;

        output.textContent = '';
        loadingIndicator.classList.remove('hidden');
        cancelButton.classList.remove('hidden');
        controls.classList.add('hidden');

        currentController = new AbortController();

        try {
            // ATUALIZAÇÃO: Enviando os padrões de exclusão no corpo da requisição
            const response = await fetch('/api/process', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    path: path,
                    ignore_patterns: ignorePatterns // Envia o array de padrões
                }),
                signal: currentController.signal,
            });

            if (!response.ok) {
                const errorText = await response.text();
                throw new Error(errorText || `Erro HTTP: ${response.status}`);
            }

            const data = await response.json();
            output.textContent = data.content;
            tokenCountSpan.textContent = data.token_count.toLocaleString('pt-BR');
            controls.classList.remove('hidden');

        } catch (error) {
            if (error.name === 'AbortError') {
                output.textContent = 'Análise cancelada.';
            } else {
                output.textContent = `Falha na requisição:\n\n${error.message}`;
            }
        } finally {
            loadingIndicator.classList.add('hidden');
            cancelButton.classList.add('hidden');
            currentController = null;
        }
    });


    copyButton.addEventListener('click', () => {
        navigator.clipboard.writeText(output.textContent).then(() => {
            copyButton.textContent = 'Copiado!';
            setTimeout(() => { copyButton.textContent = 'Copiar Tudo'; }, 2000);
        });
    });

    cancelButton.addEventListener('click', () => {
        if (currentController) {
            currentController.abort();
        }
    });
});