async function fetchStats() {
    try {
        const response = await fetch('/api/stats');
        const data = await response.json();

        document.getElementById('current-block-value').innerText = data.total_blocks;
        document.getElementById('transactions-processed-value').innerText = data.total_transactions;
        document.getElementById('shards-value').innerText = data.num_shards;
        document.getElementById('tx-pool-size-value').innerText = data.transaction_pool_size;
        document.getElementById('block-size-value').innerText = `${data.avg_block_size} bytes`;
    } catch (error) {
        console.error('Error fetching stats:', error);
    }
}

setInterval(fetchStats, 5000);
fetchStats();
