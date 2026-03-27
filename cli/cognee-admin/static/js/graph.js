// vis-network initialization helper for cognee-admin graph page
// This supplements the inline script in graph.rs for any reusable graph utilities.

/**
 * Create a vis-network instance in the given container.
 * @param {HTMLElement} container
 * @param {Object} data - { nodes: [...], edges: [...] }
 * @param {Object} [options] - vis-network options override
 * @returns {vis.Network}
 */
function createGraph(container, data, options) {
    const defaultOptions = {
        physics: {
            solver: 'forceAtlas2Based',
            forceAtlas2Based: {
                gravitationalConstant: -50,
                centralGravity: 0.01,
                springLength: 100,
                springConstant: 0.08,
            },
            stabilization: { iterations: 150 },
        },
        interaction: {
            hover: true,
            tooltipDelay: 200,
            zoomView: true,
            dragNodes: true,
            navigationButtons: false,
            keyboard: { enabled: true },
        },
        layout: {
            improvedLayout: true,
        },
        nodes: {
            font: { color: '#e5e7eb', size: 12, face: 'Inter' },
            shape: 'dot',
            size: 15,
            borderWidth: 2,
        },
        edges: {
            color: { color: '#4b5563', hover: '#00d9ff', highlight: '#00d9ff' },
            font: { color: '#6b7280', size: 10, face: 'Inter' },
            arrows: { to: { enabled: true, scaleFactor: 0.8 } },
            smooth: { type: 'continuous' },
        },
    };

    const mergedOptions = Object.assign({}, defaultOptions, options || {});
    return new vis.Network(container, data, mergedOptions);
}

/**
 * Map a node type to a color.
 */
function nodeTypeColor(type) {
    const colors = {
        entity: '#22d3ee',
        concept: '#a78bfa',
        document: '#fbbf24',
        chunk: '#4ade80',
        relation: '#f472b6',
    };
    return colors[(type || '').toLowerCase()] || '#9ca3af';
}
