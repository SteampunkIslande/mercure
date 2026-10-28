/** SMAUG administration: conserve l'API et le générateur de tableaux existants. */
(function () {
    "use strict";

    async function loadRuns(status, emptyMessage) {
        const prefix = status.toLowerCase();
        const table = document.getElementById(`${prefix}-runs-table`);
        const overflow = document.getElementById(`${prefix}-runs-overflow`);
        if (!table) return;

        try {
            const response = await fetch(`/smaug/api/listruns?page=1&page_size=10&status=${encodeURIComponent(status)}`);
            if (!response.ok) throw new Error(`HTTP ${response.status}`);
            const json = await response.json();
            if (!isApiSuccess(json)) throw new Error(getApiMessage(json) || 'Erreur inconnue');

            const data = getApiData(json);
            if (!data || !Array.isArray(data.table) || !Array.isArray(data.header)) {
                throw new Error('Format de réponse inattendu');
            }
            if (data.table.length) {
                generateJsonTable(data, table.id);
            } else {
                const message = document.createElement('p');
                message.className = 'sa-empty';
                message.textContent = emptyMessage;
                table.replaceChildren(message);
            }
            if (overflow) {
                overflow.hidden = !(data.pagination && data.pagination.total_pages > 1);
            }
        } catch (err) {
            const message = document.createElement('p');
            message.className = 'sa-empty';
            message.textContent = 'Impossible de charger les runs.';
            table.replaceChildren(message);
            showMessage('error', `Chargement ${status} : ${err.message || err}`);
        }
    }

    document.addEventListener('DOMContentLoaded', function () {
        loadRuns('Running', 'Aucun run en cours');
        loadRuns('Failure', 'Aucun run en échec');
    });
})();
