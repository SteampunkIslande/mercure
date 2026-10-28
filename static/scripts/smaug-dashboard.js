/**
 * SMAUG /smaug/home : comportement de l'interface connectée.
 * Les droits et la liste des groupes viennent de Rust (`user_groups`, `is_admin`).
 * Les formulaires et les runs sont récupérés par les API SMAUG existantes.
 * Aucun lancement d'analyse ni aucune autorisation n'est géré côté navigateur.
 */
(() => {
  'use strict';

  const config = window.smaugDashboardConfig;
  if (!config || !Array.isArray(config.userGroups)) {
    console.error('Configuration SMAUG manquante.');
    return;
  }

  const groups = config.userGroups.map(group => ({ id: String(group.id), name: group.name }));
  if (config.isAdmin) groups.push({ id: 'None', name: 'Formulaires sans groupe' });

  const formList = document.getElementById('formList');
  const groupTabs = document.getElementById('groupTabs');
  const searchInput = document.getElementById('formSearch');
  const globalSearch = document.getElementById('globalSearch');
  const count = document.getElementById('availableCount');
  const reset = document.getElementById('resetFilters');
  const noGroup = document.getElementById('no-user-group');
  const formsByGroup = new Map();
  const failedGroups = new Set();
  const categoryFeedback = document.getElementById('categoryFeedback');
  let activeGroup = groups[0]?.id ?? null;

  // Les catégories sont des raccourcis visuels, pas de nouvelles permissions.
  const categoryHints = {
    demultiplexage: ['demultiplex', 'demux', 'bcl'],
    genetique: ['geneti', 'germinal', 'constitution'],
    onco: ['onco', 'hemato', 'somati'],
    immunologie: ['immun'],
    autres: ['outils', 'autres', 'qc', 'qualite', 'utilitaire']
  };

  function normalize(text) {
    return String(text ?? '').normalize('NFD').replace(/[\u0300-\u036f]/g, '').toLowerCase();
  }

  function formUrl(form) {
    // Conserve exactement le schéma des routes existantes de SMAUG.
    switch (form.form_id) {
      case 'GitBranchAndPath': {
        const branch = String(form.branch ?? '');
        const filePath = String(form.file_path ?? '');
        if (!branch || !filePath) return null;
        // Encoder les segments sans altérer les séparateurs du chemin relatif.
        const safeBranch = encodeURIComponent(branch);
        const safePath = filePath.split('/').map(encodeURIComponent).join('/');
        return `/smaug/runs/submit/branch/${safeBranch}/${safePath}`;
      }
      // Le backend actuellement déployé n'expose pas de route GET pour GitCommit.
      // Ne pas afficher de lien de lancement vers une URL inexistante.
      default:
        return null;
    }
  }

  function showError(message) {
    // Une réponse d'erreur distante reste du texte, jamais du HTML injecté.
    const target = document.getElementById('error-message');
    if (target) {
      target.textContent = message;
      target.style.display = 'block';
    } else {
      console.error(message);
    }
  }

  function setActiveTab(id) {
    activeGroup = id;
    renderTabs();
    renderForms();
  }

  function renderTabs() {
    groupTabs.replaceChildren();
    for (const group of groups) {
      const tab = document.createElement('button');
      tab.type = 'button';
      tab.setAttribute('role', 'tab');
      tab.setAttribute('aria-selected', String(group.id === activeGroup));
      tab.textContent = group.name;
      tab.addEventListener('click', () => setActiveTab(group.id));
      groupTabs.append(tab);
    }
  }

  function showEmpty(message) {
    const empty = document.createElement('p');
    empty.className = 'empty-state';
    empty.textContent = message;
    formList.append(empty);
  }

  function renderForms() {
    formList.replaceChildren();
    const search = normalize(searchInput.value.trim());
    const forms = formsByGroup.get(activeGroup) ?? [];
    const matching = forms.filter(form => normalize(`${form.name ?? ''} ${form.description ?? ''}`).includes(search));
    count.textContent = `${matching.length} formulaire${matching.length !== 1 ? 's' : ''}`;
    reset.hidden = !search;

    if (activeGroup === null) {
      showEmpty('Aucun groupe de formulaires autorisé pour votre compte.');
      return;
    }
    if (!formsByGroup.has(activeGroup)) {
      showEmpty('Chargement des formulaires…');
      return;
    }
    if (failedGroups.has(activeGroup)) {
      showEmpty('Chargement impossible pour ce groupe. Réessayez en actualisant la page.');
      return;
    }
    if (!matching.length) {
      showEmpty('Aucun formulaire disponible pour ce groupe et cette recherche.');
      return;
    }

    for (const form of matching) {
      const item = document.createElement('div');
      item.className = 'form-item';
      const copy = document.createElement('div');
      copy.className = 'form-copy';
      const title = document.createElement('strong');
      title.textContent = form.name ?? 'Formulaire sans nom';
      const description = document.createElement('p');
      description.textContent = form.description ?? '';
      copy.append(title, description);

      const url = formUrl(form);
      if (url) {
        const launch = document.createElement('a');
        launch.href = url;
        launch.className = 'run-button';
        launch.innerHTML = '<svg class="icon" aria-hidden="true"><use href="#i-play"></use></svg> Lancer un run';
        launch.setAttribute('aria-label', `Lancer un run : ${form.name ?? 'formulaire'}`);
        item.append(copy, launch);
      } else {
        const unavailable = document.createElement('span');
        unavailable.className = 'soft-badge';
        unavailable.textContent = 'Type de formulaire non pris en charge';
        item.append(copy, unavailable);
      }
      formList.append(item);
    }
  }

  async function loadForms(group) {
    try {
      const response = await fetch(`/smaug/api/groups/forms/${group.id}`);
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      const result = await response.json();
      if (!isApiSuccess(result)) throw new Error(getApiMessage(result) || 'Erreur inconnue');
      const forms = getApiData(result)?.[group.id === 'None' ? 'without_group' : 'with_group'];
      formsByGroup.set(group.id, Array.isArray(forms) ? forms : []);
      failedGroups.delete(group.id);
    } catch (error) {
      formsByGroup.set(group.id, []);
      failedGroups.add(group.id);
      showError(`Chargement des formulaires impossible : ${error.message}`);
    }
    renderForms();
  }

  async function loadRuns() {
    const target = document.getElementById('latest-runs-table');
    try {
      const response = await fetch('/smaug/api/listruns?page=1&page_size=5');
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      const result = await response.json();
      if (!isApiSuccess(result)) throw new Error(getApiMessage(result) || 'Erreur inconnue');
      const data = getApiData(result);
      if (!data || !Array.isArray(data.table) || data.table.length === 0) {
        target.textContent = 'Aucun run récent à afficher.';
      } else {
        // Le composant existant conserve liens, colonnes et classes de statut SMAUG.
        generateJsonTable(data, 'latest-runs-table');
      }
    } catch (error) {
      target.textContent = 'Impossible de charger les runs récents.';
      showError(`Chargement des runs impossible : ${error.message}`);
    }
  }

  function scrollToForms() {
    document.getElementById('forms')?.scrollIntoView({ behavior: 'smooth', block: 'start' });
  }

  function chooseCategory(category) {
    const hints = categoryHints[category] ?? [];
    // N'afficher que des groupes transmis par /smaug/home et chargés via ses API.
    // Les mots-clés servent uniquement à la navigation : ils ne créent aucun droit.
    const matches = text => hints.some(hint => normalize(text).includes(hint));
    const byGroupName = groups.find(group => matches(group.name));
    const byForm = groups.find(group => (formsByGroup.get(group.id) ?? []).some(form =>
      matches(`${form.name ?? ''} ${form.description ?? ''}`)));
    const group = byForm ?? byGroupName;
    searchInput.value = '';
    if (globalSearch) globalSearch.value = '';
    if (group) {
      categoryFeedback.hidden = true;
      setActiveTab(group.id);
    } else {
      // Ne jamais choisir arbitrairement le premier groupe : cela présenterait
      // un formulaire potentiellement sans rapport avec la catégorie demandée.
      categoryFeedback.textContent = 'Aucun formulaire de cette catégorie n’est identifié parmi les groupes accessibles. Vous pouvez consulter les formulaires disponibles ci-dessous.';
      categoryFeedback.hidden = false;
      renderForms();
    }
    scrollToForms();
  }

  function syncSearch(value) {
    categoryFeedback.hidden = true;
    searchInput.value = value;
    if (globalSearch) globalSearch.value = value;
    // Recherche transversale, sans interroger de groupes non autorisés.
    const needle = normalize(value.trim());
    if (needle) {
      const match = groups.find(group => (formsByGroup.get(group.id) ?? []).some(form =>
        normalize(`${form.name ?? ''} ${form.description ?? ''}`).includes(needle)));
      if (match) activeGroup = match.id;
    }
    renderTabs();
    renderForms();
  }

  searchInput.addEventListener('input', () => syncSearch(searchInput.value));
  globalSearch?.addEventListener('input', () => {
    syncSearch(globalSearch.value);
    if (globalSearch.value.trim().length >= 2) scrollToForms();
  });
  reset.addEventListener('click', () => syncSearch(''));
  document.querySelectorAll('[data-category]').forEach(card => {
    card.addEventListener('click', () => chooseCategory(card.dataset.category));
  });
  document.querySelectorAll('[data-target]').forEach(button => {
    button.addEventListener('click', () => document.getElementById(button.dataset.target)?.scrollIntoView({ behavior: 'smooth' }));
  });

  if (groups.length === 0 && !config.isAdmin) noGroup.hidden = false;
  renderTabs();
  renderForms();
  Promise.allSettled(groups.map(loadForms)).then(() => renderForms());
  loadRuns();
})();
