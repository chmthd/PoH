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
                    <td>${tx.block_number}</td>
                    <td>${tx.shard_number}</td> <!-- Include Shard Number -->
                `;
                transactionsList.appendChild(row);
            });
        });

        // display shard information
        const shardInfoContainer = document.getElementById('shard-info');
        shardInfoContainer.innerHTML = '';
        data.shard_info.forEach(info => {
            shardInfoContainer.innerHTML += `
                <p>Shard ${info.id}: ${info.ip}:${info.port}</p>
            `;
        });

    } catch (error) {
        console.error('Error fetching stats:', error);
    }
}

setInterval(fetchStats, 5000);
fetchStats();
