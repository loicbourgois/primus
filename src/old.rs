use reqwest;
use serde::{Deserialize, Serialize};
use serde_json::Result;
use std::collections::HashMap;
use std::collections::HashSet;
use std::format as f;
use std::fs;
use std::path::Path;
use std::println as p;
use std::time::Instant;
use urlencoding;
mod eta;
use eta::ETA;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_read_file() {
        let result = read_file("lib.rs");
    }
}

static APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"), "/2");

fn read_file(path: &str) -> String {
    // p!("reading {path}");
    fs::read_to_string(path).unwrap()
}

fn read_file_2(path: &str) -> core::result::Result<String, std::io::Error> {
    // p!("reading {path}");
    fs::read_to_string(path)
}

#[derive(Debug)]
pub enum MyError {
    io_error(std::io::Error),
    serde_json_error(serde_json::Error),
}

pub fn read_json_2<T>(path: &str) -> core::result::Result<T, MyError>
where
    T: for<'a> Deserialize<'a>,
{
    match read_file_2(path) {
        Ok(c) => match serde_json::from_str::<T>(&c) {
            Ok(r) => Ok(r),
            Err(e) => Err(MyError::serde_json_error(e)),
        },
        Err(e) => Err(MyError::io_error(e)), //#serde_json::Error { err: e },
    }
}

fn write_file(path: &str, content: &str) {
    // p!("writing {path}");
    fs::write(path, content).unwrap();
}

pub struct Puller {
    pub query_name: HashSet<String>,
    pub root_folder: String,
    pub exclude: HashSet<String>,
}

impl Puller {
    pub fn pull_wikidata(&mut self, name: &str, query_: &str) {
        let query = &query_
            .replace('\n', " ")
            .trim()
            .split(' ')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        let digest = sha256::digest(query);
        p!("pulling wikidata/{name} | wikidata_strict/{digest}");
        if self.exclude.get(name).is_some() {
            p!("  skipping {name}: excluded in config");
            return;
        }
        let root_folder = &self.root_folder;
        if self.query_name.get(name).is_some() {
            p!("Already pulled in this run: '{name}'");
            return;
        } else {
            self.query_name.insert(name.to_string());
        }
        if Path::new(&f!("{root_folder}/wikidata_strict/{digest}/result.json")).exists()
            && Path::new(&f!("{root_folder}/wikidata/{name}/result.json")).exists()
            && read_file(&f!("{root_folder}/wikidata_strict/{digest}/result.json"))
                == read_file(&f!("{root_folder}/wikidata/{name}/result.json"))
        {
            p!("  skipping {name}: results already pulled");
            return;
        }
        fs::create_dir_all(&f!("{root_folder}/wikidata_strict/{digest}")).unwrap();
        fs::create_dir_all(&f!("{root_folder}/wikidata/{name}")).unwrap();
        write_file(
            &f!("{root_folder}/wikidata_strict/{digest}/query.sparql"),
            query,
        );
        write_file(&f!("{root_folder}/wikidata/{name}/query.sparql"), query);
        let query_encoded = urlencoding::encode(query);
        let url = f!("https://query.wikidata.org/sparql?query={query_encoded}&format=json");
        let client = reqwest::blocking::Client::builder()
            .user_agent(APP_USER_AGENT)
            .build()
            .unwrap();
        match client.get(url).send() {
            Ok(response) => {
                write_file(
                    &f!("{root_folder}/wikidata_strict/{digest}/response.json"),
                    &f!("{response:#?}"),
                );
                write_file(
                    &f!("{root_folder}/wikidata/{name}/response.json"),
                    &f!("{response:#?}"),
                );
                let status = response.status();
                if status == 200 {
                    match response.text() {
                        Ok(result) => {
                            write_file(
                                &f!("{root_folder}/wikidata_strict/{digest}/result.json"),
                                &result,
                            );
                            write_file(&f!("{root_folder}/wikidata/{name}/result.json"), &result);
                        }
                        Err(e) => {
                            p!("  error: status: {status} | response.text empty");
                        }
                    }
                } else {
                    p!("  error: status: {status}");
                }
            }
            Err(e) => {
                p!("  error: {e}");
            }
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub root: String,
    pub exclude: HashSet<String>,
    pub direct_attributs: HashSet<String>,
    pub child_attributs: HashSet<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct WikidataResults {
    pub head: HashMap<String, Vec<String>>,
    pub results: WikidataResultsInner,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct WikidataResultsInner {
    pub bindings: Vec<HashMap<String, HashMap<String, String>>>,
}

pub fn read_json<T>(path: &str) -> T
where
    T: for<'a> Deserialize<'a>,
{
    serde_json::from_str::<T>(&read_file(path)).unwrap()
}

pub fn go_2(root: &str, root_folder: &str) -> Result<()> {
    let mut puller = Puller {
        query_name: HashSet::new(),
        root_folder: root_folder.to_string(),
        exclude: HashSet::new(),
    };
    puller.pull_wikidata(
        &f!("instance_count/{root}"),
        &f!(r#"
            SELECT ?class (COUNT(*) AS ?count)
            WHERE
            {{
                ?class wdt:P279* wd:{root}.
                ?instance wdt:P31 ?class.
            }}
            GROUP BY ?class
            order by desc( ?count )
        "#),
    );
    let classes = read_json::<WikidataResults>(&f!(
        "{root_folder}/wikidata/instance_count/{root}/result.json"
    ));
    let l1 = classes.results.bindings.len();
    let mut instance_ids = HashSet::new();
    for (i1, r1) in classes.results.bindings.iter().enumerate() {
        p!("{i1}/{l1}");
        let v = r1["class"]["value"].replace("http://www.wikidata.org/entity/", "");
        let k = "P31";
        puller.pull_wikidata(
            &f!("ikv/_/{k}/{v}"),
            &f!(r#"
                SELECT ?i ?k ?v
                WHERE
                {{
                    ?i wdt:{k} wd:{v}.
                    BIND("{v}" AS ?v) .
                    BIND("{k}" AS ?k) .
                }}
            "#),
        );
        match read_json_2::<WikidataResults>(&f!(
            "{root_folder}/wikidata/ikv/_/{k}/{v}/result.json"
        )) {
            Ok(instances) => {
                let l2 = instances.results.bindings.len();
                for (i2, r2) in instances.results.bindings.iter().enumerate() {
                    let i = r2["i"]["value"].replace("http://www.wikidata.org/entity/", "");
                    instance_ids.insert(i);
                }
            }
            Err(e) => {
                p!("  error: {e:?}");
            }
        }
    }
    let l2 = instance_ids.len();
    let mut eta2 = ETA::new(l2);
    for (i2, iid) in instance_ids.iter().enumerate() {
        eta2.tick();
        p!("{eta2}");
        let i = iid;
        puller.pull_wikidata(
            &f!("ikvkv/{i}/_/_/_/_"),
            &f!(r#"
                SELECT ?i ?k ?v ?k2 ?v2
                WHERE
                {{
                    wd:{i} ?k_ ?v.
                    ?v ?k2_ ?v2 .
                    BIND("{i}" AS ?i) .
                    ?k wikibase:directClaim ?k_ .
                    ?k2 wikibase:directClaim ?k2_ .
                }}
            "#),
        );
    }
    Ok(())
}

pub fn go(input: &str, root_folder: &str) -> Result<()> {
    let config = read_json::<Config>(input);
    let root = config.root.clone();
    fs::create_dir_all(&f!("{root_folder}")).unwrap();
    p!("root_folder: {root_folder}");
    let lang = "en";
    let mut puller = Puller {
        query_name: HashSet::new(),
        root_folder: root_folder.to_string(),
        exclude: config.exclude,
    };
    puller.pull_wikidata(
        &f!("all_class/{root}"),
        &f!(r#"
            SELECT ?class
            WHERE
            {{
                ?class wdt:P279* wd:{root}.
            }}
        "#),
    );
    puller.pull_wikidata(
        &f!("all_class_label/{root}/{lang}"),
        &f!(r#"
            SELECT ?class ?class_label (lang(?class_label) as ?lang)
            WHERE
            {{
                ?class wdt:P279* wd:{root}.
                ?class rdfs:label ?class_label filter (lang(?class_label) = "{lang}").
            }}
        "#),
    );
    puller.pull_wikidata(
        &f!("hierarchy/{root}"),
        &f!(r#"
            SELECT ?parent_class ?class
            WHERE
            {{
                ?class wdt:P279* wd:{root}.
                ?parent_class wdt:P279 ?class.
            }}
        "#),
    );
    // go_2();
    // puller.pull_wikidata(
    //     &f!("instance_count/{root}"),
    //     &f!(r#"
    //         SELECT ?class (COUNT(*) AS ?count)
    //         WHERE
    //         {{
    //             ?class wdt:P279* wd:{root}.
    //             ?instance wdt:P31 ?class.
    //         }}
    //         GROUP BY ?class
    //         order by desc( ?count )
    //     "#),
    // );
    // let classes = read_json::<WikidataResults>(&f!(
    //     "{root_folder}/wikidata/instance_count/{root}/result.json"
    // ));
    // let l1 = classes.results.bindings.len();
    // let mut instance_ids = HashSet::new();
    // for (i1, r1) in classes.results.bindings.iter().enumerate() {
    //     p!("{i1}/{l1}");
    //     let v = r1["class"]["value"].replace("http://www.wikidata.org/entity/", "");
    //     let k = "P31";
    //     puller.pull_wikidata(
    //         &f!("ikv/_/{k}/{v}"),
    //         &f!(r#"
    //             SELECT ?i ?k ?v
    //             WHERE
    //             {{
    //                 ?i wdt:{k} wd:{v}.
    //                 BIND("{v}" AS ?v) .
    //                 BIND("{k}" AS ?k) .
    //             }}
    //         "#),
    //     );
    //     match read_json_2::<WikidataResults>(&f!(
    //         "{root_folder}/wikidata/ikv/_/{k}/{v}/result.json"
    //     )) {
    //         Ok(instances) => {
    //             let l2 = instances.results.bindings.len();
    //             for (i2, r2) in instances.results.bindings.iter().enumerate() {
    //                 let i = r2["i"]["value"].replace("http://www.wikidata.org/entity/", "");
    //                 instance_ids.insert(i);
    //             }
    //         }
    //         Err(e) => {
    //             p!("  error: {e:?}");
    //         }
    //     }
    // }
    // // let l2 = instance_ids.len();
    // // let mut eta2 = ETA::new(l2);
    // // for (i2, iid) in instance_ids.iter().enumerate() {
    // //     eta2.tick();
    // //     p!("{eta2}");
    // //     let i = iid;
    // //     puller.pull_wikidata(
    // //         &f!("ikv/{i}/_/_"),
    // //         &f!(r#"
    // //             SELECT ?i ?k ?v
    // //             WHERE
    // //             {{
    // //                 wd:{i} ?k_ ?v.
    // //                 BIND("{i}" AS ?i) .
    // //                 ?k wikibase:directClaim ?k_ .
    // //             }}
    // //         "#),
    // //     );
    // // }
    // let l2 = instance_ids.len();
    // let mut eta2 = ETA::new(l2);
    // for (i2, iid) in instance_ids.iter().enumerate() {
    //     eta2.tick();
    //     p!("{eta2}");
    //     let i = iid;
    //     puller.pull_wikidata(
    //         &f!("ikvkv/{i}/_/_/_/_"),
    //         &f!(r#"
    //             SELECT ?i ?k ?v ?k2 ?v2
    //             WHERE
    //             {{
    //                 wd:{i} ?k_ ?v.
    //                 ?v ?k2_ ?v2 .
    //                 BIND("{i}" AS ?i) .
    //                 ?k wikibase:directClaim ?k_ .
    //                 ?k2 wikibase:directClaim ?k2_ .
    //             }}
    //         "#),
    //     );
    // }

    // select ?i ?k ?v ?k2 ?v2
    // where {
    //     wd:Q44578 ?k_ ?v .
    //     ?v ?k2_ ?v2 .
    //     bind("Q44578" as ?i) .
    //     ?k wikibase:directClaim ?k_ .
    //     ?k2 wikibase:directClaim ?k2_ .
    // }

    // let l2 = config.direct_attributs.len();
    // let l3 = config.child_attributs.len();
    // let classes = read_json::<WikidataResults>(&f!(
    //     "{root_folder}/wikidata/instance_count/{root}/result.json"
    // ));
    // let l1 = classes.results.bindings.len();
    // for (i1, r1) in classes.results.bindings.iter().enumerate() {
    //     let k1 = r1["class"]["value"].replace("http://www.wikidata.org/entity/", "");
    //     p!("{i1}/{l1}");
    //     for (i2, k2) in config.direct_attributs.iter().enumerate() {
    //         p!("{i1}/{l1} > {i2}/{l2}");
    //         puller.pull_wikidata(
    //             &f!("field_value_direct/{k1}/{k2}"),
    //             &f!(r#"
    //                 SELECT distinct ?item ?field_k ?value
    //                 WHERE
    //                 {{
    //                     ?item wdt:P31 wd:{k1}.
    //                     ?item wdt:{k2} ?value .
    //                     BIND("{root}" AS ?root_class) .
    //                     BIND("{k2}" AS ?field) .
    //                 }}
    //             "#),
    //         );
    //         for (i3, k3) in config.child_attributs.iter().enumerate() {
    //             p!("{i1}/{l1} > {i2}/{l2} > {i3}/{l3}");
    //             puller.pull_wikidata(
    //                 &f!("child_field_value_direct/{k1}/{k2}/{k3}"),
    //                 &f!(r#"
    //                     SELECT distinct ?item ?field ?value
    //                     WHERE
    //                     {{
    //                         ?item_ wdt:P31 wd:{k1}.
    //                         ?item_ wdt:{k2} ?item .
    //                         ?item wdt:{k3} ?value .
    //                         BIND("{k3}" AS ?field) .
    //                     }}
    //                 "#),
    //             );
    //         }
    //     }
    // }

    // for (i1, k1) in config.direct_attributs.iter().enumerate() {
    //     p!("{i1}/{l1}");
    //     puller.pull_wikidata(
    //         &f!("field_value/{root}/{k1}"),
    //         &f!(r#"
    //             SELECT distinct ?item ?field ?value
    //             WHERE
    //             {{
    //                 ?class wdt:P279* wd:{root}.
    //                 ?item wdt:P31 ?class.
    //                 ?item wdt:{k1} ?value .
    //                 BIND("{k1}" AS ?field) .
    //             }}
    //         "#),
    //     );
    //     for (i2, k2) in config.child_attributs.iter().enumerate() {
    //         p!("{i1}/{l1} > {i2}/{l2}");
    //         puller.pull_wikidata(
    //             &f!("child_field_value/{root}/{k1}/{k2}"),
    //             &f!(r#"
    //                 SELECT distinct ?item ?field ?value
    //                 WHERE
    //                 {{
    //                     ?class wdt:P279* wd:{root}.
    //                     ?item1 wdt:P31 ?class.
    //                     ?item1 wdt:{k1} ?item .
    //                     ?item wdt:{k2} ?value .
    //                     BIND("{k2}" AS ?field) .
    //                 }}
    //             "#),
    //         );
    //     }
    // }

    // puller.pull_wikidata(
    //     &f!("instance_count/{root}"),
    //     &f!(r#"
    //         SELECT ?class (COUNT(*) AS ?count)
    //         WHERE
    //         {{
    //             ?class wdt:P279* wd:{root}.
    //             ?instance wdt:P31 ?class.
    //         }}
    //         GROUP BY ?class
    //         order by desc( ?count )
    //     "#),
    // );
    // puller.pull_wikidata(
    //     &f!("property_list/{root}"),
    //     &f!(r#"
    //         SELECT distinct ?property
    //         WHERE
    //         {{
    //             ?class wdt:P279* wd:{root}.
    //             ?instance wdt:P31 ?class.
    //             ?instance ?property_k ?property_v .
    //             ?property wikibase:directClaim ?property_k .
    //         }}
    //     "#),
    // );
    // puller.pull_wikidata(
    //     &f!("property_list_direct/{root}"),
    //     &f!(r#"
    //         SELECT distinct ?property
    //         WHERE
    //         {{
    //             ?class wdt:P279 wd:{root}.
    //             ?instance wdt:P31 ?class.
    //             ?instance ?property_k ?property_v.
    //             ?property wikibase:directClaim ?property_k .
    //         }}
    //     "#),
    // );
    // let aa = read_json::<WikidataResults>(&f!(
    //     "{root_folder}/wikidata/property_list_direct/{root}/result.json"
    // ));
    // p!(
    //     "{:?}",
    //     aa.results
    //         .bindings
    //         .iter()
    //         .map(|x| x["property"]["value"].replace("http://www.wikidata.org/entity/", ""))
    //         .collect::<Vec<_>>()
    // );

    // puller.pull_wikidata(
    //     &f!("property_label/{root}/{lang}"),
    //     &f!(r#"
    //         SELECT ?property ?property_label (lang(?property_label) as ?lang)
    //         with {{
    //             SELECT distinct ?property
    //             WHERE
    //             {{
    //                 ?class wdt:P279* wd:{root}.
    //                 ?instance wdt:P31 ?class.
    //                 ?property_v ?property_k ?instance.
    //                 ?property wikibase:directClaim ?property_k .
    //             }}
    //         }} as %q1
    //         where {{
    //             include %q1
    //             ?property rdfs:label ?property_label filter (lang(?property_label) = "en").
    //         }}
    //     "#),
    // );
    // let classes = read_json::<WikidataResults>(&f!(
    //     "{root_folder}/wikidata/instance_count/{root}/result.json"
    // ));
    // let properties = read_json::<WikidataResults>(&f!(
    //     "{root_folder}/wikidata/property_list/{root}/result.json"
    // ));
    // let l = classes.results.bindings.len();
    // for (i, r) in classes.results.bindings.iter().enumerate() {
    //     p!("{i}/{l}");
    //     let class = r["class"]["value"].replace("http://www.wikidata.org/entity/", "");
    //     puller.pull_wikidata(
    //         &f!("instance_list/{class}"),
    //         &f!(r#"
    //             SELECT distinct ?instance
    //             WHERE
    //             {{
    //                 ?instance wdt:P31 wd:{class}.
    //             }}
    //         "#),
    //     );
    //     let l2 = properties.results.bindings.len();
    //     for (i2, r2) in properties.results.bindings.iter().enumerate() {
    //         p!("{i}/{l} > {i2}/{l2}");
    //         let property = r2["property"]["value"].replace("http://www.wikidata.org/entity/", "");
    //         puller.pull_wikidata(
    //             &f!("instance_property/{class}/{property}"),
    //             &f!(r#"
    //                 SELECT distinct ?instance ?property_v
    //                 WHERE
    //                 {{
    //                     ?instance wdt:P31 wd:{class} .
    //                     ?instance wdt:{property} ?property_v .
    //                 }}
    //             "#),
    //         );
    //         puller.pull_wikidata(
    //             &f!("instance_property_property2/{class}/{property}"),
    //             &f!(r#"
    //                 SELECT distinct ?instance ?property_k ?property_v ?property_2_k ?property_2_v
    //                 WHERE
    //                 {{
    //                     ?instance wdt:P31 wd:{class} .
    //                     ?instance wdt:{property} ?property_v .
    //                     ?property_v ?property_2_k_ ?property_2_v .
    //                     ?property_2_k wikibase:directClaim ?property_2_k_ .
    //                     ?property_k wikibase:directClaim wdt:{property} .
    //                 }}
    //             "#),
    //         );
    //     }
    //     // puller.pull_wikidata(
    //     //     &f!("instance/{class}/field"),
    //     //     &f!(r#"
    //     //         SELECT distinct ?instance ?field_k ?field_v
    //     //         WHERE
    //     //         {{
    //     //             ?instance wdt:P31 wd:{class} .
    //     //             ?instance ?field_k  ?field_v .
    //     //         }}
    //     //     "#),
    //     // );
    // }
    Ok(())
}
