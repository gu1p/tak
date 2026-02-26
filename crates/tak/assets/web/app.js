(function () {
  const summary = document.getElementById("summary");
  const selection = document.getElementById("selection");
  const networkContainer = document.getElementById("network");

  async function loadGraph() {
    const response = await fetch("/graph.json", { cache: "no-store" });
    if (!response.ok) {
      throw new Error(`failed to load graph.json (${response.status})`);
    }
    return response.json();
  }

  function nodeTitle(node) {
    return [
      `label: ${node.label}`,
      `package: ${node.package}`,
      `task: ${node.task}`,
      `deps: ${node.deps}`,
      `dependents: ${node.dependents}`,
    ].join("\n");
  }

  function render(graph) {
    const nodes = new vis.DataSet(
      graph.nodes.map((node) => ({
        id: node.id,
        label: node.label,
        title: nodeTitle(node),
        shape: "dot",
        size: 16 + Math.min(node.dependents * 2, 14),
      }))
    );

    const edges = new vis.DataSet(
      graph.edges.map((edge) => ({
        from: edge.from,
        to: edge.to,
        arrows: "to",
      }))
    );

    const data = { nodes, edges };
    const options = {
      autoResize: true,
      interaction: {
        hover: true,
        navigationButtons: true,
        keyboard: true,
      },
      physics: {
        enabled: true,
        stabilization: {
          enabled: true,
          iterations: 200,
        },
      },
      nodes: {
        borderWidth: 1,
        color: {
          border: "#3d88ff",
          background: "#4fb3ff",
          highlight: {
            border: "#93d9ff",
            background: "#66ecff",
          },
        },
        font: {
          color: "#f2f8ff",
          size: 13,
          face: "IBM Plex Sans",
        },
      },
      edges: {
        color: {
          color: "#6f87a7",
          highlight: "#80e8ff",
        },
        smooth: {
          enabled: true,
          type: "dynamic",
        },
      },
    };

    const network = new vis.Network(networkContainer, data, options);

    summary.textContent =
      `nodes=${graph.nodes.length}, edges=${graph.edges.length}` +
      (graph.target ? `, target=${graph.target}` : ", target=(all)");

    network.on("click", (event) => {
      if (!event.nodes || event.nodes.length === 0) {
        selection.textContent = "Click a node to inspect task details.";
        return;
      }

      const id = event.nodes[0];
      const node = graph.nodes.find((item) => item.id === id);
      if (!node) {
        selection.textContent = `Selected node ${id} (details unavailable)`;
        return;
      }

      selection.textContent = [
        `label: ${node.label}`,
        `package: ${node.package}`,
        `task: ${node.task}`,
        `deps: ${node.deps}`,
        `dependents: ${node.dependents}`,
      ].join("\n");
    });
  }

  loadGraph()
    .then(render)
    .catch((error) => {
      summary.textContent = "Failed to render graph";
      selection.textContent = error.stack || String(error);
    });
})();
