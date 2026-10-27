use specimen_kernel::language_v3::{script::*, *};
fn run(text: &str, row: Option<(UtilityKind,UtilityEffect,&str)>, conditions: Vec<(String,bool)>) -> Prepared {
 let mut snaps=UtilityKind::ALL.into_iter().map(|kind| UtilitySnapshot{population:"p".into(),version:"v".into(),kind,rows:vec![]}).collect::<Vec<_>>();
 if let Some((kind,effect,phrase))=row {snaps.iter_mut().find(|s|s.kind==kind).unwrap().rows.push(UtilityRow{reference:"r".into(),phrase:phrase.into(),effect});}
 let tables=UtilityTables::new("p","v",snaps).unwrap();
 prepare(Input{id:"x".into(),text:text.into()}, &ContextSnapshot{population:"p".into(),utility_version:"v".into(),tokens:vec![],conditions},&tables,Budget::default(),&mut FixedDraw(0)).unwrap()
}
fn guard_probes(){
 let r=Registry::new("r",1,vec![],vec![Declaration{name:"Text".into(),aliases:vec![],scopes:vec![Scope::NodeAction],reads:vec![],handler:Handler::Text}]).unwrap();
 let c=r.capabilities(vec![]).unwrap();let s=compile("Text(\"effect\")",Scope::NodeAction,&r).unwrap();
 for (label,p) in [
 ("quoted condition case mismatch",run("if \"ALLOW\" then go",None,vec![("\"allow\"".into(),true)])),
 ("grammar negation",run("halt go",Some((UtilityKind::Grammar,UtilityEffect::Role(Role::Negation),"halt")),vec![])),
 ("conjunction shadows builtin negation",run("please don't go",Some((UtilityKind::Conjunction,UtilityEffect::Conjunction(Connective::And),"don't")),vec![]))] {
 let a=&p.clauses().last().unwrap().variants()[0];
 println!("{label}: original={:?}, inhibited={}, executable={}, ast={:?}, execution={:?}",a.original(),a.inhibited(),a.executable(),a.ast(),execute_for_action(&s,a,&r,&c,ScriptBudget::default()).map(|p|p.value().clone()));
 }
}

fn main(){
 guard_probes();
 let p=run("alpha, beta gamma delta epsilon zeta",None,vec![]);
 let a=&p.clauses()[0].variants()[0];
 let k=specimen_kernel::text_v3::encode(a).unwrap();
 println!("complexity disagreement: getter={}, key_count={}, class={:?}",a.complexity_token_count(),k.token_count(),k.resolution());
 let r=Registry::new("r",1,vec![],vec![Declaration{name:"my-text".into(),aliases:vec![],scopes:vec![Scope::NodeAction],reads:vec![],handler:Handler::Text}]).unwrap();
 println!("accepted hyphenated declaration compile: {:?}",compile("my-text(\"x\")",Scope::NodeAction,&r));
 use specimen_kernel::{identity::*,scanner_v3::routes::*};
 let mut table=HybridTable::default();let pop=Population::new("p","s","text");
 for i in 1..=257u128 {
  table.insert(EntryRecord{population:pop.clone(),node:NodeRef(i+1000),reference:EntryRef(i),key:k.pattern_key().clone(),original:format!("entry{i}"),ast:ActionAst::default(),resource:None},None).unwrap();
 }
 let r=Registry::new("r",1,vec![],vec![]).unwrap(); let caps=r.capabilities(vec![]).unwrap();
 let index=RouteIndex::new(pop.clone(),vec![Scope::NodeAction],&r).unwrap();
 let empty=run("",None,vec![]);
 println!("empty query, 257 records, work_steps=1: {:?}",candidates(&table,&index,&pop,Scope::NodeAction,&empty,&r,&caps,RouteBudget{work_steps:1,..RouteBudget::default()}).map(|cs|cs.len()));
}
