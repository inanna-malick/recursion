use crate::map_layer::MapLayer;

type VizNodeId = u32;

pub enum VizAction {
    // expand a seed to a node, with new child seeds if any
    ExpandSeed {
        target_id: VizNodeId,
        txt: String,
        seeds: Vec<(VizNodeId, String)>,
    },
    // collapse node to value, removing all child nodes
    CollapseNode {
        target_id: VizNodeId,
        txt: String,
    },
}

pub struct Viz {
    seed_txt: String,
    root_id: VizNodeId,
    actions: Vec<VizAction>,
}

pub fn serialize_html(v: Viz) -> serde_json::Result<String> {
    let mut out = String::new();
    out.push_str(TEMPLATE_BEFORE);
    out.push_str(&serialize_json(v)?);
    out.push_str(TEMPLATE_AFTER);

    Ok(out)
}

pub fn serialize_json(v: Viz) -> serde_json::Result<String> {
    use serde_json::value::Value;
    let mut actions = v.actions;
    // sort to mimic non-fused ana -> cata style eval
    actions.sort_by_key(|x| match x {
        VizAction::ExpandSeed {
            target_id,
            txt,
            seeds,
        } => 0,
        VizAction::CollapseNode { target_id, txt } => 1,
    });
    let actions: Vec<Value> = actions
        .into_iter()
        .map(|elem| match elem {
            VizAction::ExpandSeed {
                target_id,
                txt,
                seeds,
            } => {
                let mut h = serde_json::Map::new();
                h.insert(
                    "target_id".to_string(),
                    Value::String(target_id.to_string()),
                );
                h.insert("txt".to_string(), Value::String(txt));
                let mut json_seeds = Vec::new();
                for (node_id, txt) in seeds.into_iter() {
                    let mut h = serde_json::Map::new();
                    h.insert("node_id".to_string(), Value::String(node_id.to_string()));
                    h.insert("txt".to_string(), Value::String(txt));
                    json_seeds.push(Value::Object(h));
                }
                h.insert("seeds".to_string(), Value::Array(json_seeds));
                Value::Object(h)
            }
            VizAction::CollapseNode { target_id, txt } => {
                let mut h = serde_json::Map::new();
                h.insert(
                    "target_id".to_string(),
                    Value::String(target_id.to_string()),
                );
                h.insert("txt".to_string(), Value::String(txt));
                Value::Object(h)
            }
        })
        .collect();

    let viz_root = {
        let mut h = serde_json::Map::new();
        h.insert("node_id".to_string(), Value::String(v.root_id.to_string()));
        h.insert("txt".to_string(), Value::String(v.seed_txt));
        h.insert("typ".to_string(), Value::String("seed".to_string()));
        Value::Object(h)
    };

    let viz_js = {
        let mut h = serde_json::Map::new();
        h.insert("root".to_string(), viz_root);
        h.insert("actions".to_string(), Value::Array(actions));
        Value::Object(h)
    };

    serde_json::to_string(&viz_js)
}

// use std::fmt::Debug;
// TODO: split out root seed case to separate field on return obj, not needed as part of enum!
pub fn expand_and_collapse_v<Seed, Out, Expandable, Collapsable>(
    seed: Seed,
    mut coalg: impl FnMut(Seed) -> Expandable,
    mut alg: impl FnMut(Collapsable) -> Out,
) -> (Out, Viz)
where
    Expandable: MapLayer<(), Unwrapped = Seed>,
    <Expandable as MapLayer<()>>::To:
        MapLayer<Out, Unwrapped = (), To = Collapsable> + std::fmt::Debug,
    Seed: std::fmt::Debug,
    Out: std::fmt::Debug,
{
    enum State<Pre, Post> {
        PreVisit(Pre),
        PostVisit(Post),
    }

    let mut keygen = 1; // 0 is used for root node
    let mut v = Vec::new();
    let root_seed_txt = format!("{:?}", seed);

    let mut vals: Vec<Out> = vec![];
    let mut todo: Vec<State<(VizNodeId, Seed), _>> = vec![State::PreVisit((0, seed))];

    while let Some(item) = todo.pop() {
        match item {
            State::PreVisit((viz_node_id, seed)) => {
                let mut seeds_v = Vec::new();

                let node = coalg(seed);
                let mut topush = Vec::new();
                let node = node.map_layer(|seed| {
                    let k = keygen;
                    keygen += 1;
                    seeds_v.push((k, format!("{:?}", seed)));

                    topush.push(State::PreVisit((k, seed)))
                });

                v.push(VizAction::ExpandSeed {
                    target_id: viz_node_id,
                    txt: format!("{:?}", node),
                    seeds: seeds_v,
                });

                todo.push(State::PostVisit((viz_node_id, node)));
                todo.extend(topush.into_iter());
            }
            State::PostVisit((viz_node_id, node)) => {
                let node = node.map_layer(|_: ()| vals.pop().unwrap());

                let out = alg(node);

                v.push(VizAction::CollapseNode {
                    target_id: viz_node_id,
                    txt: format!("{:?}", out),
                });

                vals.push(out)
            }
        };
    }
    (
        vals.pop().unwrap(),
        Viz {
            seed_txt: root_seed_txt,
            root_id: 0,
            actions: v,
        },
    )
}

static TEMPLATE_BEFORE: &'static str = r###"
<!DOCTYPE html>
<meta charset="UTF-8">
<style>

.node rect {
  fill: #fff;
  stroke-width: 4px;
 }

 .node text {
  font: 13px verdana;
}

body {
  background-color: skyblue;
} 


.link {
  fill: none;
  stroke-width: 4px;
}

</style>

<body>

<!-- load the d3.js library -->	
<script src="https://d3js.org/d3.v7.js"></script>
<script>


 // colors for use in nodes, links, etc
 const collapse_stroke = "mediumVioletRed";
 const expand_stroke = "steelBlue";
 const structure_stroke = "black";


const data = "###;

static TEMPLATE_AFTER: &'static str = r###"

 var treeData = data.root;

 var actions = data.actions;


// Set the dimensions and margins of the diagram
var margin = {top: 20, right: 90, bottom: 30, left: 90},
    width = 1560 - margin.left - margin.right,
    height = 500 - margin.top - margin.bottom;

// append the svg object to the body of the page
// appends a 'group' element to 'svg'
// moves the 'group' element to the top left margin
var svg = d3.select("body").append("svg")
    .attr("width", width + margin.right + margin.left)
    .attr("height", height + margin.top + margin.bottom)
    .append("g")
    .attr("transform", "translate("
          + margin.left + "," + margin.top + ")");

var i = 0,
    duration = 750,
    root;

// declares a tree layout and assigns the size
var treemap = d3.tree().size([height, width]);

// Assigns parent, children, height, depth
 root = d3.hierarchy(treeData, function(d) { return d.children; });
 root.x0 = height / 2;
root.y0 = 0;


update(root);

var pause = 2;


 let intervalId = setInterval(function () {
    if (pause == 0) {
     var next = actions.shift();
     if (next) {
    if (next.seeds) { // in this case, is expand (todo explicit typ field for this)

        let target = root.find(x => x.data.node_id == next.target_id);

        target.data.txt = next.txt;
        target.data.typ = "structure";

        if (next.seeds.length) {
            target.children = [];
            target.data.children = [];
        } else {
            delete target.children;
            delete target.data.children;
        }
        next.seeds.forEach(function(seed) {
            var newNode = d3.hierarchy(seed);
            newNode.depth = target.depth + 1;
            newNode.height = target.height - 1;
            newNode.parent = target;

            newNode.data.typ = "seed";

            target.children.push(newNode);
            target.data.children.push(newNode.data);
        });

        update(target);

    } else { // in this case, is collapse
        let target = root.find(x => x.data.node_id == next.target_id);

        // remove child nodes from tree
        delete target.children;
        delete target.data.children;
        target.data.txt = next.txt;
        target.data.typ = "collapse";


        update(target);

     }
     } else {
         clearInterval(intervalId);
     }} else { pause -= 1;}
 }, 2000);




function update(source) {

  // Assigns the x and y position for the nodes
  var treeData = treemap(root);

  // Compute the new tree layout.
  var nodes = treeData.descendants(),
      links = treeData.descendants().slice(1);

  // Normalize for fixed-depth.
  nodes.forEach(function(d){ d.y = d.depth * 180});

  // ****************** Nodes section ***************************

  // Update the nodes...
  var node = svg.selectAll('g.node')
      .data(nodes, function(d) {return d.id || (d.id = ++i); });

  // Enter any new modes at the parent's previous position.
  var nodeEnter = node.enter().append('g')
      .attr('class', 'node')
      .attr("transform", function(d) {
        return "translate(" + source.y0 + "," + source.x0 + ")";
    });

  // Add rect for the nodes
  nodeEnter.append('rect')
      .attr('class', 'node')
      .attr('width', 1e-6)
      .attr('height', 1e-6)
           .transition()
           .duration(500)

           .transition()
           .duration(500)
     ;

  // Add labels for the nodes
  nodeEnter.append('text')
      .attr("dy", ".35em")
      .attr("x", function(d) {
          return d.children || d._children ? -13 : 13;
      })
      .attr("text-anchor", function(d) {
          return d.children || d._children ? "end" : "start";
      })
           .text(function(d) { return (d.data.typ + ":" + d.data.txt); });

  // UPDATE
  var nodeUpdate = nodeEnter.merge(node);

  // Transition to the proper position for the node
  nodeUpdate.transition()
    .duration(duration)
    .attr("transform", function(d) {
        return "translate(" + d.y + "," + d.x + ")";
     });

  // Update the node attributes and style
     nodeUpdate.select('rect.node')
               .attr('stroke', function(d) {
                   switch(d.data.typ) {
                       case 'structure':
                           return structure_stroke;
                       case 'seed':
                           return expand_stroke;
                       case 'collapse':
                           return collapse_stroke;
                   }
               })
            .attr('width', function(d){ return textSize(d.data.txt + ":" + d.data.typ).width})
            .attr('height', textSize("x").height + 5 )
               .attr("transform", function(d) {return "translate(0, -" + (textSize("x").height + 5) / 2 + ")"; })
            .transition()
     .duration(500);

     // update text
     nodeUpdate.select("text")
                            .text(function(d) { return (d.data.typ + ":" + d.data.txt); });


  // Remove any exiting nodes
  var nodeExit = node.exit().transition()
      .duration(duration)
      .attr("transform", function(d) {
          return "translate(" + source.y + "," + source.x + ")";
      })
      .remove();

  // On exit reduce the node circles size to 0
    nodeExit.select('rect')
      .attr('width', 1e-6)
      .attr('height', 1e-6);

  // On exit reduce the opacity of text labels
  nodeExit.select('text')
    .style('fill-opacity', 1e-6);

  // ****************** links section ***************************

  // Update the links...
  var link = svg.selectAll('path.link')
      .data(links, function(d) { return d.id; });

  // Enter any new links at the parent's previous position.
  var linkEnter = link.enter().insert('path', "g")
      .attr("class", "link")
      .attr('d', function(d){
        var o = {x: source.x0, y: source.y0}
        return diagonal(o, o)
      });

  // UPDATE
  var linkUpdate = linkEnter.merge(link);

  // Transition back to the parent element position
     linkUpdate.transition()
               .attr('stroke', function(d) {
                   switch(d.data.typ) {
                       case 'structure':
                           return structure_stroke;
                       case 'seed':
                           return expand_stroke;
                       case 'collapse':
                           return collapse_stroke;
                   }})
      .duration(duration)
      .attr('d', function(d){ return diagonal(d, d.parent) });

  // Remove any exiting links
  var linkExit = link.exit().transition()
      .duration(duration)
      .attr('d', function(d) {
        var o = {x: source.x, y: source.y}
        return diagonal(o, o)
      })
      .remove();

  // Store the old positions for transition.
  nodes.forEach(function(d){
    d.x0 = d.x;
    d.y0 = d.y;
  });

  // Creates a curved (diagonal) path from parent to the child nodes
  function diagonal(s, d) {

    path = `M ${s.y} ${s.x}
            C ${(s.y + d.y) / 2} ${s.x},
              ${(s.y + d.y) / 2} ${d.x},
              ${d.y} ${d.x}`

    return path
  }
 }

 function textSize(text) {
     if (!d3) return;
     var container = d3.select('body').append('svg');
     container.append('text').attr("x", -99999).attr( "y", -99999 ).text(text);
     var size = container.node().getBBox();
     container.remove();
     return { width: size.width, height: size.height };
 }


</script>
</body>

"###;
