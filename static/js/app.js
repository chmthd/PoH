async function fetchStats() {
    try {
        const response = await fetch('/api/stats');
        const data = await response.json();

        document.getElementById('current-block-value').innerText = data.total_blocks;
        document.getElementById('transactions-processed-value').innerText = data.total_transactions;
        document.getElementById('block-size-value').innerText = `${data.avg_block_size} bytes`;
        document.getElementById('avg-tx-size-value').innerText = `${data.avg_tx_size} bytes`;

        const transactionsList = document.getElementById('transactions-list');
        transactionsList.innerHTML = '';

        data.shard_stats.forEach(shard => {
            shard.transactions.forEach(tx => {
                const row = document.createElement('tr');
                row.innerHTML = `
                    <td>${tx.id}</td>
                    <td>${tx.status}</td>
                    <td>${tx.block_number}</td> <!-- New block number column -->
                `;
                transactionsList.appendChild(row);
            });
        });
    } catch (error) {
        console.error('Error fetching stats:', error);
    }
}

setInterval(fetchStats, 5000);
fetchStats();
