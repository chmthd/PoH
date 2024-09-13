async function fetchStats() {
    try {
        const response = await fetch('/api/stats');
        const data = await response.json();

        // Update general stats
        document.getElementById('current-block-value').innerText = data.total_blocks;
        document.getElementById('transactions-processed-value').innerText = data.total_transactions;
        document.getElementById('block-size-value').innerText = `${data.avg_block_size} bytes`;
        document.getElementById('avg-tx-size-value').innerText = `${data.avg_tx_size} bytes`;

        // Update transactions
        const transactionsList = document.getElementById('transactions-list');
        transactionsList.innerHTML = '';

        data.shard_stats.forEach(shard => {
            shard.transactions.forEach(tx => {
                const row = document.createElement('tr');
                row.innerHTML = `
                    <td>${tx.id}</td>
                    <td>${tx.status}</td>
                    <td>${tx.block_number}</td>
                    <td>${tx.shard_number}</td> 
                `;
                transactionsList.appendChild(row);
            });
        });

        // Display shard information
        const shardInfoContainer = document.getElementById('shard-info');
        shardInfoContainer.innerHTML = '';
        data.shard_info.forEach(info => {
            shardInfoContainer.innerHTML += `
                <p>Shard ${info.id}: ${info.ip}:${info.port}</p>
            `;
        });

        // Update validators
        const validatorsList = document.getElementById('validators-list');
        validatorsList.innerHTML = '';

        data.shard_stats.forEach(shard => {
            shard.validators.forEach(validator => {
                const row = document.createElement('tr');
                row.innerHTML = `
                    <td>${validator.id}</td>
                    <td>${validator.votes_cast}</td>
                    <td>${validator.successful_votes}</td>
                    <td>${validator.average_response_time_ms}</td>
                    <td>${validator.participation_count}</td>
                    <td>${validator.consensus_contribution_count}</td>
                    <td>${validator.epochs_active}</td>
                    <td>${validator.final_vote_weight}</td>
                `;
                validatorsList.appendChild(row);
            });
        });

    } catch (error) {
        console.error('Error fetching stats:', error);
    }
}

async function fetchNodes() {
    try {
        const response = await fetch('/api/nodes');
        const nodes = await response.json();

        console.log('Nodes received from server:', nodes); 

        const nodesList = document.getElementById('nodes-list');
        nodesList.innerHTML = '';

        nodes.forEach(node => {
            const listItem = document.createElement('li');
            listItem.innerText = `Node ${node[0]}: ${node[1]}`;
            nodesList.appendChild(listItem);
        });
    } catch (error) {
        console.error('Error fetching nodes:', error);
    }
}

// Periodically refresh stats and node information
setInterval(() => {
    fetchStats();
    fetchNodes();
}, 5000);

fetchStats();
fetchNodes();
