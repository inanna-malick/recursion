use std::fmt::Display;

use recursion::{Collapsible, Expandable, MappableFrame};

pub trait CollapsibleVizExt: Collapsible
where
    Self: Sized + Display,
    <Self::FrameToken as MappableFrame>::Frame<()>: Display,
{
    type Viz: VizOut;


    fn collapse_frames_v<Out>(
        self,
        collapse_frame: impl FnMut(<Self::FrameToken as MappableFrame>::Frame<Out>) -> Out,
    ) -> (Out, Self::Viz)
    where
        Out: Display;

    fn try_collapse_frames_v<Out, E: Display>(
        self,
        collapse_frame: impl FnMut(<Self::FrameToken as MappableFrame>::Frame<Out>) -> Result<Out, E>,
    ) -> (Result<Out, E>, Self::Viz)
    where
        Out: Display;
}


pub trait ExpandableVizExt: Expandable
where
    Self: Sized + Display,
    <Self::FrameToken as MappableFrame>::Frame<()>: Display,
{
    // '()' if disabled
    type Viz: VizOut;

    fn expand_frames_v<In>(
        input: In,
        expand_frame: impl FnMut(In) -> <Self::FrameToken as MappableFrame>::Frame<In>,
    ) -> (Self, Self::Viz)
    where
        In: Display;
}


pub trait VizOut {
    type Config;

    fn label(self, info_header: String, info_txt: String) -> Self;

    fn fuse(self, next: Self, info_header: String, info_txt: String) -> Self;

    fn finish(self, cfg: Self::Config);
}


#[cfg(not(enabled))]
impl<X: Collapsible> CollapsibleVizExt for X
where
    Self: Sized + Display,
    <Self::FrameToken as MappableFrame>::Frame<()>: Display,
{

    type Viz = ();

    fn collapse_frames_v<Out>(
        self,
        collapse_frame: impl FnMut(<Self::FrameToken as MappableFrame>::Frame<Out>) -> Out,
    ) -> (Out, Self::Viz)
    where
        Out: Display,
    {
        let res = recursion::frame::expand_and_collapse::<Self::FrameToken, Self, Out>(self, Self::into_frame, collapse_frame);
        (res, ())
    }

    fn try_collapse_frames_v<Out, E: Display>(
        self,
        collapse_frame: impl FnMut(<Self::FrameToken as MappableFrame>::Frame<Out>) -> Result<Out, E>,
    ) -> (Result<Out, E>, Self::Viz)
    where
        Out: Display,
    {
        let res = recursion::frame::try_expand_and_collapse::<Self::FrameToken, Self, Out, E>(
            self,
            |x| Ok(Self::into_frame(x)),
            collapse_frame,
        );

        (res, ())
    }
}

impl VizOut for () {
    type Config = ();

    fn label(self, info_header: String, info_txt: String) -> Self {
        ()
    }

    fn fuse(self, next: Self, info_header: String, info_txt: String) -> Self {
        ()
    }

    fn finish(self, cfg: Self::Config) {
        ()
    }
}


#[cfg(enabled)]
impl<X: Collapsible> CollapsibleVizExt for X
where
    Self: Sized + Display,
    <Self::FrameToken as MappableFrame>::Frame<()>: Display,
{

    type Viz = Viz;

    fn collapse_frames_v<Out>(
        self,
        collapse_frame: impl FnMut(<Self::FrameToken as MappableFrame>::Frame<Out>) -> Out,
    ) -> (Out, Self::Viz)
    where
        Out: Display,
    {
        expand_and_collapse_v::<Self::FrameToken, Self, Out>(self, Self::into_frame, collapse_frame)
    }

    fn try_collapse_frames_v<Out, E: Display>(
        self,
        collapse_frame: impl FnMut(<Self::FrameToken as MappableFrame>::Frame<Out>) -> Result<Out, E>,
    ) -> (Result<Out, E>, Self::Viz)
    where
        Out: Display,
    {
        try_expand_and_collapse_v::<Self::FrameToken, Self, Out, E>(
            self,
            |x| Ok(Self::into_frame(x)),
            collapse_frame,
        )
    }
}

#[cfg(enabled)]
impl<X: Expandable> ExpandableVizExt for X
where
    Self: Sized + Display,
    <Self::FrameToken as MappableFrame>::Frame<()>: Display,
{
    type Viz = Viz;

    fn expand_frames_v<In>(
        input: In,
        expand_frame: impl FnMut(In) -> <Self::FrameToken as MappableFrame>::Frame<In>,
    ) -> (Self, Viz)
    where
        In: Display,
    {
        expand_and_collapse_v::<Self::FrameToken, In, Self>(input, expand_frame, Self::from_frame)
    }
}

type VizNodeId = u32;

#[cfg(enabled)]
#[derive(Clone)]
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
    // info text display!
    InfoCard {
        info_header: String,
        info_txt: String,
    },
}

#[cfg(enabled)]
#[derive(Clone)]
pub struct Viz {
    seed_txt: String,
    root_id: VizNodeId,
    actions: Vec<VizAction>,
}

#[cfg(enabled)]
impl VizOut for Viz {
    type Config = String;

    fn label(mut self, info_header: String, info_txt: String) -> Self {
        let mut actions = vec![VizAction::InfoCard {
            info_header,
            info_txt,
        }];
        actions.extend(self.actions.into_iter());
        self.actions = actions;

        self
    }

    fn fuse(self, next: Self, info_header: String, info_txt: String) -> Self {
        let mut actions = self.actions;
        actions.push(VizAction::InfoCard {
            info_txt,
            info_header,
        });
        actions.extend(next.actions.into_iter());

        Self {
            seed_txt: self.seed_txt,
            root_id: self.root_id,
            actions,
        }
    }

    fn finish(self, cfg: Self::Config) {
        let to_write = serialize_html(self).unwrap();

        println!("write to path: {:?}", cfg);

        std::fs::write(cfg, to_write).unwrap();
    }
}

#[cfg(enabled)]
// this is hilariously jamky and I can do better, but this is an experimental feature so I will not prioritize doing so.
pub fn serialize_html(v: Viz) -> serde_json::Result<String> {
    let mut out = String::new();
    out.push_str(TEMPLATE_BEFORE);
    out.push_str(&serialize_json(v)?);
    out.push_str(TEMPLATE_AFTER);

    Ok(out)
}

#[cfg(enabled)]
pub fn serialize_json(v: Viz) -> serde_json::Result<String> {
    use serde_json::value::Value;
    let actions: Vec<Value> = v
        .actions
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
            VizAction::InfoCard {
                info_txt,
                info_header,
            } => {
                let mut h = serde_json::Map::new();
                h.insert("info_txt".to_string(), Value::String(info_txt.to_string()));
                h.insert(
                    "info_header".to_string(),
                    Value::String(info_header.to_string()),
                );
                h.insert("typ".to_string(), Value::String("info_card".to_string()));
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

#[cfg(enabled)]
pub fn try_expand_and_collapse_v<F, Seed, Out, E>(
    seed: Seed,
    mut coalg: impl FnMut(Seed) -> Result<F::Frame<Seed>, E>,
    mut alg: impl FnMut(F::Frame<Out>) -> Result<Out, E>,
) -> (Result<Out, E>, Viz)
where
    F: MappableFrame,
    E: Display,
    F::Frame<()>: Display,
    Seed: Display,
    Out: Display,
{
    enum State<Pre, Post> {
        PreVisit(Pre),
        PostVisit(Post),
    }

    let mut keygen = 1; // 0 is used for root node
    let mut v = Vec::new();
    let root_seed_txt = format!("{}", seed);

    let mut vals: Vec<Out> = vec![];
    let mut todo: Vec<State<(VizNodeId, Seed), _>> = vec![State::PreVisit((0, seed))];

    while let Some(item) = todo.pop() {
        match item {
            State::PreVisit((viz_node_id, seed)) => {
                let mut seeds_v = Vec::new();

                let node = match coalg(seed) {
                    Ok(node) => node,
                    Err(e) => {
                        v.push(VizAction::InfoCard {
                            info_header: "Error during expand!".to_string(),
                            info_txt: format!("error: {}", e),
                        });
                        return (
                            Err(e),
                            Viz {
                                seed_txt: root_seed_txt,
                                root_id: 0,
                                actions: v,
                            },
                        );
                    }
                };
                let mut topush = Vec::new();
                let node = F::map_frame(node, |seed| {
                    let k = keygen;
                    keygen += 1;
                    seeds_v.push((k, format!("{}", seed)));

                    topush.push(State::PreVisit((k, seed)))
                });

                v.push(VizAction::ExpandSeed {
                    target_id: viz_node_id,
                    txt: format!("{}", node),
                    seeds: seeds_v,
                });

                todo.push(State::PostVisit((viz_node_id, node)));
                todo.extend(topush.into_iter());
            }
            State::PostVisit((viz_node_id, node)) => {
                let node = F::map_frame(node, |_: ()| vals.pop().unwrap());

                let out = match alg(node) {
                    Ok(out) => out,
                    Err(e) => {
                        v.push(VizAction::InfoCard {
                            info_header: "Error during collapse!".to_string(),
                            info_txt: format!("error: {}", e),
                        });
                        return (
                            Err(e),
                            Viz {
                                seed_txt: root_seed_txt,
                                root_id: 0,
                                actions: v,
                            },
                        );
                    }
                };

                v.push(VizAction::CollapseNode {
                    target_id: viz_node_id,
                    txt: format!("{}", out),
                });

                vals.push(out)
            }
        };
    }

    let out = vals.pop().unwrap();

    v.push(VizAction::InfoCard {
        info_header: "Completed".to_string(),
        info_txt: format!("result: {}", out),
    });

    (
        Ok(out),
        Viz {
            seed_txt: root_seed_txt,
            root_id: 0,
            actions: v,
        },
    )
}

#[cfg(enabled)]
// use std::fmt::Debug;
// TODO: split out root seed case to separate field on return obj, not needed as part of enum!
pub fn expand_and_collapse_v<F, Seed, Out>(
    seed: Seed,
    mut coalg: impl FnMut(Seed) -> F::Frame<Seed>,
    mut alg: impl FnMut(F::Frame<Out>) -> Out,
) -> (Out, Viz)
where
    F: MappableFrame,
    // F::Frame<Seed>: Display,
    F::Frame<()>: Display,
    Seed: Display,
    Out: Display,
{
    enum State<Pre, Post> {
        PreVisit(Pre),
        PostVisit(Post),
    }

    let mut keygen = 1; // 0 is used for root node
    let mut v = Vec::new();
    let root_seed_txt = format!("{}", seed);

    let mut vals: Vec<Out> = vec![];
    let mut todo: Vec<State<(VizNodeId, Seed), _>> = vec![State::PreVisit((0, seed))];

    while let Some(item) = todo.pop() {
        match item {
            State::PreVisit((viz_node_id, seed)) => {
                let mut seeds_v = Vec::new();

                let node = coalg(seed);
                let mut topush = Vec::new();
                let node = F::map_frame(node, |seed| {
                    let k = keygen;
                    keygen += 1;
                    seeds_v.push((k, format!("{}", seed)));

                    topush.push(State::PreVisit((k, seed)))
                });

                v.push(VizAction::ExpandSeed {
                    target_id: viz_node_id,
                    txt: format!("{}", node),
                    seeds: seeds_v,
                });

                todo.push(State::PostVisit((viz_node_id, node)));
                todo.extend(topush.into_iter());
            }
            State::PostVisit((viz_node_id, node)) => {
                let node = F::map_frame(node, |_: ()| vals.pop().unwrap());

                let out = alg(node);

                v.push(VizAction::CollapseNode {
                    target_id: viz_node_id,
                    txt: format!("{}", out),
                });

                vals.push(out)
            }
        };
    }

    let out = vals.pop().unwrap();

    v.push(VizAction::InfoCard {
        info_header: "Completed".to_string(),
        info_txt: format!("result: {}", out),
    });

    (
        out,
        Viz {
            seed_txt: root_seed_txt,
            root_id: 0,
            actions: v,
        },
    )
}

#[cfg(enabled)]
//TODO/FIXME: something better than this. that said, this is in experimental so :shrug_emoji:
static TEMPLATE_BEFORE: &'static str = r###"
<!DOCTYPE html>
<meta charset="UTF-8">
<style>

.node rect {
  fill: #fff;
  stroke-width: 4px;
  rx: 4px;
  rY: 4px;
 }

 .node text {
  font: 16px verdana;
}

body {
  background-color: lightcyan;
} 

.infocard {
  background-color: white;
  border-style: solid;
  width: 500px;
  padding: 10px;
  border-radius: 10px;
} 

.infocard .cardheader {
  font-size: 25px;
  padding-top: 5px;
  padding-bottom: 5px;
  border-bottom: solid;
  border-width: 5px;
} 

.infocard .cardbody {
  font-size: 15px;
  padding: 10px;
  font-family: "Lucida Console", "Courier New", monospace;
  background-color: steelblue;
  color: white;
}

.link {
  fill: none;
  stroke-width: 4px;
}

</style>

<body>

<div opacity="0" id="titlecard" class="infocard">
  <div class="cardheader">header</div>
  <div class="cardbody">body</div>
</div>

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
var margin = {top: 0, right: 10, bottom: 30, left: 30},
    width = 900 - margin.left - margin.right,
    height = 410 - margin.top - margin.bottom;

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
    duration = 250,
    root;

// declares a tree layout and assigns the size
var treemap = d3.tree().size([height/1.4, width]);

// Assigns parent, children, height, depth
 root = d3.hierarchy(treeData, function(d) { return d.children; });
 root.x0 = height / 2;
root.y0 = 0;


update(root);

var pause = 0;


 let intervalId = setInterval(function () {
    if (pause == 0) {
     var next = actions.shift();
     if (next) {
    if (next.typ == "info_card") {
         d3.select("#titlecard .cardheader").text(next.info_header);
         d3.select("#titlecard .cardbody").text(next.info_txt);

         d3.select("#titlecard")
         .transition().duration(500)
         .style("border-color", "mediumvioletred")
         .style("color", "mediumvioletred")
         .transition().duration(1000)
         .style("border-color", "black")
         .style("color", "black");

    } else if (next.seeds) { // in this case, is expand (todo explicit typ field for this)

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
 }, 600);

function update(source) {

  // Assigns the x and y position for the nodes
  var treeData = treemap(root);

  // Compute the new tree layout.
  var nodes = treeData.descendants(),
      links = treeData.descendants().slice(1);

  // Normalize for fixed-depth.
  nodes.forEach(function(d){ d.y = d.depth * 110});

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
           .duration(duration)

           .transition()
           .duration(duration)
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
           .text(function(d) { return (d.data.txt); });

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
            .attr('width', function(d){ return textSize(d.data.txt).width})
            .attr('height', textSize("x").height + 5 )
               .attr("transform", function(d) {return "translate(0, -" + (textSize("x").height + 5) / 2 + ")"; })
            .transition()
     .duration(duration);

     // update text
     nodeUpdate.select("text")
                            .text(function(d) { return (d.data.txt); });


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
     return { width: size.width + 30, height: size.height + 10 };
 }


</script>
</body>

"###;
