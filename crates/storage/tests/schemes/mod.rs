use opencanvas_storage::*;
use serde_json::json;

pub fn fixture() -> Vec<Operation> {
    serde_json::from_value(json!([
        {"op":"upsert_token","token":{"id":"ink","name":"Ink","type":"color","value":{"default":"#111111","themes":{"night":"#eeeeee","day":"#008800"}}}},
        {"op":"upsert_token","token":{"id":"alias","name":"Alias","type":"color","value":{"aliasId":"ink"}}},
        {"op":"upsert_style","style":{"id":"paint","name":"Paint","kind":"fill","value":{"tokenId":"alias"}}},
        {"op":"upsert_style","style":{"id":"copy","name":"Copy","kind":"text","value":{"fontId":"font_bundled","fontSize":14,"lineHeight":20,"fill":{"tokenId":"alias"}}}},
        {"op":"create_node","id":"page","kind":"page","parentId":null,"props":{}},
        {"op":"create_node","id":"source","kind":"frame","parentId":"page","props":{"width":400,"height":300,"theme":"day"}},
        {"op":"create_node","id":"leaf","kind":"frame","parentId":"source","props":{"component":{"name":"Leaf"},"width":20,"height":20,"fill":{"tokenId":"ink"}}},
        {"op":"create_node","id":"master","kind":"frame","parentId":"source","props":{"component":{"name":"Tile"},"width":100,"height":80,"fill":{"tokenId":"ink"},"stroke":{"color":{"tokenId":"ink"},"width":2,"align":"outside"}}},
        {"op":"create_node","id":"box","kind":"rectangle","parentId":"master","props":{"width":20,"height":20,"fill":{"styleId":"paint"},"stroke":{"color":{"tokenId":"ink"},"width":2,"align":"center"},"shadow":[{"x":2,"color":{"tokenId":"ink"}},{"y":4,"color":"#000"}],"states":{"hover":{"fill":{"tokenId":"ink"},"opacity":0.4,"stroke":{"color":{"tokenId":"ink"},"width":3}},"focus":{"stroke":{"color":{"tokenId":"ink"},"width":4}},"pressed":{"opacity":0.2}}}},
        {"op":"create_node","id":"label","kind":"text","parentId":"master","props":{"text":"source","styleId":"copy"}},
        {"op":"create_node","id":"nested","kind":"instance","parentId":"master","props":{"componentId":"leaf"}},
        {"op":"create_node","id":"inner","kind":"frame","parentId":"master","props":{"width":10,"height":10,"theme":"default","fill":{"tokenId":"ink"}}},
        {"op":"create_node","id":"frame","kind":"frame","parentId":"page","props":{"width":400,"height":300,"theme":"night"}},
        {"op":"create_node","id":"one","kind":"instance","parentId":"frame","props":{"componentId":"master","fill":"#abcdef","stroke":{"color":"#123456"},"radius":8,"opacity":0.9,"shadow":[{"color":{"tokenId":"ink"}}],"overrides":{"box":{"stroke":{"color":"#f00"},"shadow":[{"blur":6,"color":{"tokenId":"alias"}}],"states":{"hover":{"fill":"#00f","stroke":{"color":"#0f0"}}},"radius":[1,2],"opacity":0.7},"label":{"text":"placed","styleId":"copy"}}}},
        {"op":"create_node","id":"two","kind":"instance","parentId":"frame","props":{"componentId":"master","theme":"day"}}
    ])).unwrap()
}
