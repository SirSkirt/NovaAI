use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::nova::cognition::{CognitiveRuntimeReport, ExperienceRoute, ModuleContribution};
use crate::nova::ids::stable_hash_id;
use crate::nova::learning::RustLearningState;
use crate::nova::semantics::CanonicalCognitiveState;

const DEVELOPMENT_SCHEMA: u32 = 1;
const MAX_EPISODES: usize = 240;
const MAX_CATEGORIES: usize = 600;
const MAX_RELATIONS: usize = 1200;
const MAX_ABSTRACTIONS: usize = 300;
const MAX_ROUTES: usize = 160;
const CONSOLIDATE_EVERY: u64 = 8;

fn now_ms() -> u128 { super::mind::unix_ms() }
fn canon(value: &str) -> String { value.trim().to_ascii_lowercase().replace('-', " ").replace('_', " ").split_whitespace().collect::<Vec<_>>().join(" ") }
fn unique(mut values: Vec<String>, max: usize) -> Vec<String> { let mut seen=BTreeSet::new(); values.retain(|v| { let k=canon(v); !k.is_empty() && seen.insert(k) }); values.truncate(max); values }
fn add_terms(out: &mut Vec<String>, value: &str) { for part in value.split(|c:char| !c.is_alphanumeric() && c!='_' && c!='-') { let k=canon(part); if k.len()>1 { out.push(k); } } }
fn overlap(a:&[String], b:&[String]) -> usize { let b:BTreeSet<_>=b.iter().map(|v|canon(v)).collect(); a.iter().filter(|v| b.contains(&canon(v))).count() }

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all="camelCase")]
pub struct DevelopmentCategory {
    pub id:String, pub name:String, pub key:String, pub origin:String, pub status:String,
    #[serde(default)] pub traits:Vec<Value>, #[serde(default)] pub evidence:Vec<Value>,
    pub support:u64, pub created_at:u128, pub updated_at:u128,
}
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all="camelCase")]
pub struct DevelopmentCategoryRelation {
    pub id:String, pub subject:String, pub predicate:String, pub object:String, pub origin:String,
    #[serde(default)] pub evidence:Vec<Value>, pub support:u64, pub created_at:u128, pub updated_at:u128,
}
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all="camelCase")]
pub struct DevelopmentEpisode {
    pub id:String, pub turn_id:String, pub prompt_hash:String, pub prompt_preview:String, pub supported:bool,
    #[serde(default)] pub concept_terms:Vec<String>, #[serde(default)] pub goal_terms:Vec<String>, #[serde(default)] pub relation_terms:Vec<String>,
    #[serde(default)] pub contributors:Vec<ModuleContribution>, #[serde(default)] pub experience_route_ids:Vec<String>,
    pub cycles:u32, pub created_at:u128,
}
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all="camelCase")]
pub struct ProvisionalAbstraction {
    pub id:String, pub signature:String, pub kind:String, pub status:String, pub general_term:String, pub description:String,
    #[serde(default)] pub module_numbers:Vec<u64>, #[serde(default)] pub concept_terms:Vec<String>, pub instances:u64,
    pub confidence:f32, #[serde(default)] pub evidence:Vec<Value>, pub created_at:u128, pub updated_at:u128,
}
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all="camelCase")]
pub struct DevelopmentConsolidationReport {
    pub ran:bool, pub reason:String, pub episodes_considered:usize, pub routes_merged:usize, pub routes_pruned:usize,
    pub abstractions_created:usize, pub abstractions_updated:usize, pub completed_at:u128,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all="camelCase")]
pub struct RustDevelopmentState {
    pub schema_version:u32, pub initialized:bool, pub revision:u64,
    #[serde(default)] pub categories:Vec<DevelopmentCategory>, #[serde(default)] pub relations:Vec<DevelopmentCategoryRelation>,
    #[serde(default)] pub episodes:Vec<DevelopmentEpisode>, #[serde(default)] pub abstractions:Vec<ProvisionalAbstraction>,
    pub since_consolidation:u64, pub last_consolidation:Option<DevelopmentConsolidationReport>, pub updated_at:u128,
}
impl Default for RustDevelopmentState { fn default()->Self { Self{schema_version:DEVELOPMENT_SCHEMA,initialized:false,revision:0,categories:vec![],relations:vec![],episodes:vec![],abstractions:vec![],since_consolidation:0,last_consolidation:None,updated_at:0} } }
impl RustDevelopmentState {
    pub fn new_initialized()->Self { let mut s=Self::default(); s.initialized=true; s.updated_at=now_ms(); s }
    pub fn normalize(&mut self) { self.schema_version=DEVELOPMENT_SCHEMA; self.initialized=true; self.categories.truncate(MAX_CATEGORIES); self.relations.truncate(MAX_RELATIONS); if self.episodes.len()>MAX_EPISODES { self.episodes=self.episodes.split_off(self.episodes.len()-MAX_EPISODES); } if self.abstractions.len()>MAX_ABSTRACTIONS { self.abstractions=self.abstractions.split_off(self.abstractions.len()-MAX_ABSTRACTIONS); } self.updated_at=now_ms(); }
    pub fn register_category(&mut self,name:&str,origin:&str,status:&str,traits:Vec<Value>,evidence:Vec<Value>)->bool { let key=canon(name); if key.is_empty(){return false;} let now=now_ms(); if let Some(row)=self.categories.iter_mut().find(|c|c.key==key){ let before=serde_json::to_string(row).unwrap_or_default(); row.support=row.support.saturating_add(1); row.updated_at=now; if row.origin=="bootstrap"&&origin!="bootstrap"{row.origin=origin.to_string();} if row.status=="PROVISIONAL"&&status!="PROVISIONAL"{row.status=status.to_string();} for v in traits {if !row.traits.contains(&v){row.traits.push(v);}} for v in evidence {if !row.evidence.contains(&v){row.evidence.push(v);}} row.traits.truncate(24); row.evidence.truncate(32); return before!=serde_json::to_string(row).unwrap_or_default(); }
      self.categories.push(DevelopmentCategory{id:stable_hash_id("category",&key),name:name.trim().to_string(),key,origin:origin.to_string(),status:status.to_string(),traits,evidence,support:1,created_at:now,updated_at:now}); if self.categories.len()>MAX_CATEGORIES{self.categories.remove(0);} true }
}

#[cfg(test)]
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all="camelCase")]
pub struct DevelopmentImportInput { pub persisted_state:Option<RustDevelopmentState>, #[serde(default)] pub legacy_categories:Vec<DevelopmentCategory>, #[serde(default)] pub legacy_relations:Vec<DevelopmentCategoryRelation>, #[serde(default)] pub legacy_episodes:Vec<DevelopmentEpisode>, #[serde(default)] pub legacy_abstractions:Vec<ProvisionalAbstraction>, pub reason:Option<String> }
#[cfg(test)]
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all="camelCase")]
pub struct DevelopmentImportReceipt { pub imported:bool, pub authority:String, pub state:RustDevelopmentState, pub reason:String }

#[cfg(test)]
pub fn import_development(input:DevelopmentImportInput,current:&RustDevelopmentState)->DevelopmentImportReceipt { if current.initialized {return DevelopmentImportReceipt{imported:false,authority:"rust_v7_pass_9".into(),state:current.clone(),reason:"rust_development_already_initialized".into()};} let state=if let Some(mut s)=input.persisted_state {s.normalize();s} else {let mut s=RustDevelopmentState::new_initialized();s.categories=input.legacy_categories;s.relations=input.legacy_relations;s.episodes=input.legacy_episodes;s.abstractions=input.legacy_abstractions;s.normalize();s}; DevelopmentImportReceipt{imported:true,authority:"rust_v7_pass_9".into(),reason:input.reason.unwrap_or_else(||"pass7_development_import".into()),state}
}

#[derive(Default)] struct Signature { concepts:Vec<String>, goals:Vec<String>, relations:Vec<String>, fallback:Vec<String> }
fn signature(state:&CanonicalCognitiveState)->Signature { let mut s=Signature::default(); for c in &state.active_concepts {add_terms(&mut s.concepts,&c.key);add_terms(&mut s.concepts,&c.label);} for c in &state.active_categories{add_terms(&mut s.concepts,c);} for e in &state.events{add_terms(&mut s.relations,&e.predicate_key);} for r in &state.relations{add_terms(&mut s.relations,&r.relation_type);} for q in &state.requests{add_terms(&mut s.goals,&q.request_type);add_terms(&mut s.goals,&q.answer_relation);if let Some(t)=&q.target{add_terms(&mut s.concepts,t);}} for u in &state.unresolved{add_terms(&mut s.goals,&u.need_type);} if s.concepts.is_empty(){for t in &state.surface.tokens{if t.word{add_terms(&mut s.fallback,&t.surface);}}} s.concepts=unique(s.concepts,24);s.goals=unique(s.goals,14);s.relations=unique(s.relations,18);s.fallback=unique(s.fallback,18);s }
fn route_score(route:&ExperienceRoute,s:&Signature)->usize {overlap(&route.concept_terms,&s.concepts)*14+overlap(&route.goal_terms,&s.goals)*10+overlap(&route.relation_terms,&s.relations)*6+overlap(&route.fallback_terms,&s.fallback)*2}
fn update_route(route:&mut ExperienceRoute,s:&Signature,report:&CognitiveRuntimeReport){ route.concept_terms=unique([route.concept_terms.clone(),s.concepts.clone()].concat(),24);route.goal_terms=unique([route.goal_terms.clone(),s.goals.clone()].concat(),14);route.relation_terms=unique([route.relation_terms.clone(),s.relations.clone()].concat(),18);route.fallback_terms=unique([route.fallback_terms.clone(),s.fallback.clone()].concat(),18);let mut nums=route.module_numbers.clone();let mut strengths=route.module_strengths.as_object().cloned().unwrap_or_default();for row in &report.changed{if let Some(n)=row.module_number{if !nums.contains(&n){nums.push(n);}let prior=strengths.get(&n.to_string()).and_then(Value::as_f64).unwrap_or(0.45);let contribution=(0.35+(row.delta_score.min(8) as f64)*0.07).min(1.0);strengths.insert(n.to_string(),json!((prior*0.72+contribution*0.28).min(1.0)));}}nums.truncate(48);route.module_numbers=nums;route.module_strengths=Value::Object(strengths);route.contributors=report.changed.iter().filter_map(|r|r.module_number.map(|n|ModuleContribution{module_id:r.module_id.clone(),module_number:Some(n),strength:(0.35+(r.delta_score.min(8) as f32)*0.07).min(1.0),state_change_count:r.delta_score})).collect();route.successes=route.successes.saturating_add(1);route.confidence=(route.confidence.max(0.12)+0.06).min(1.0); }
fn reflect_success(learning:&mut RustLearningState,state:&CanonicalCognitiveState,report:&CognitiveRuntimeReport)->Option<String>{if report.changed.iter().all(|r|r.module_number.is_none()){return None;}let s=signature(state);let attempted:BTreeSet<_>=report.experience_route_ids.iter().cloned().collect();let mut best:Option<(usize,usize)>=None;for(i,r)in learning.experience_routes.iter().enumerate(){let score=route_score(r,&s);let bonus=if attempted.contains(&r.id){1000}else{0};let total=score+bonus;if best.is_none_or(|(_,b)|total>b){best=Some((i,total));}}let index=if let Some((i,_score))=best.filter(|(_,score)|*score>=10){i}else{let sig=format!("{}|{}|{}",s.concepts.join(","),s.goals.join(","),s.relations.join(","));learning.experience_routes.push(ExperienceRoute{id:stable_hash_id("route",&format!("{sig}|{}",learning.revision)),concept_ids:vec![],concept_terms:s.concepts.clone(),goal_terms:s.goals.clone(),relation_terms:s.relations.clone(),fallback_terms:s.fallback.clone(),module_numbers:vec![],module_strengths:json!({}),contributors:vec![],successes:0,failures:0,confidence:0.55});learning.experience_routes.len()-1};update_route(&mut learning.experience_routes[index],&s,report);let learned_id=learning.experience_routes[index].id.clone();if learning.experience_routes.len()>MAX_ROUTES{learning.experience_routes.remove(0);}Some(learned_id)}
fn reflect_failure(learning:&mut RustLearningState,report:&CognitiveRuntimeReport){for id in &report.experience_route_ids{if let Some(r)=learning.experience_routes.iter_mut().find(|r|&r.id==id){r.failures=r.failures.saturating_add(1);r.confidence=(r.confidence-0.05).max(0.12);}}}

fn consolidate(dev:&mut RustDevelopmentState,learning:&mut RustLearningState,reason:&str)->DevelopmentConsolidationReport { let mut merged=0;let mut removed=BTreeSet::new();for i in 0..learning.experience_routes.len(){if removed.contains(&i){continue;}for j in i+1..learning.experience_routes.len(){if removed.contains(&j){continue;}let a=&learning.experience_routes[i];let b=&learning.experience_routes[j];let ca=overlap(&a.concept_terms,&b.concept_terms);let denom=a.concept_terms.len().max(b.concept_terms.len()).max(1);let ga=overlap(&a.goal_terms,&b.goal_terms);if ca*10<denom*6 || (!a.goal_terms.is_empty()&&!b.goal_terms.is_empty()&&ga==0){continue;}let (keep,drop)=if a.successes>=b.successes{(i,j)}else{(j,i)};let d=learning.experience_routes[drop].clone();let k=&mut learning.experience_routes[keep];k.concept_terms=unique([k.concept_terms.clone(),d.concept_terms].concat(),24);k.goal_terms=unique([k.goal_terms.clone(),d.goal_terms].concat(),14);k.relation_terms=unique([k.relation_terms.clone(),d.relation_terms].concat(),18);k.module_numbers.extend(d.module_numbers);k.module_numbers.sort();k.module_numbers.dedup();k.module_numbers.truncate(48);k.successes=k.successes.saturating_add(d.successes);k.failures=k.failures.saturating_add(d.failures);k.confidence=k.confidence.max(d.confidence);removed.insert(drop);merged+=1;}}
 let old=learning.experience_routes.len();learning.experience_routes=learning.experience_routes.iter().enumerate().filter(|(i,r)|!removed.contains(i)&&!(r.failures>=5&&r.confidence<0.15)).map(|(_,r)|r.clone()).collect();let pruned=old-learning.experience_routes.len();
 let mut groups:BTreeMap<String,Vec<&DevelopmentEpisode>>=BTreeMap::new();for e in dev.episodes.iter().filter(|e|e.supported&&!e.contributors.is_empty()){let goal=e.goal_terms.first().cloned().unwrap_or_else(||"general".into());let mut nums=e.contributors.iter().filter_map(|c|c.module_number).take(4).collect::<Vec<_>>();nums.sort();let key=format!("{goal}|{:?}",nums);groups.entry(key).or_default().push(e);}let mut created=0;let mut updated=0;for(sig,items)in groups{let prompts:BTreeSet<_>=items.iter().map(|e|e.prompt_hash.clone()).collect();if items.len()<3||prompts.len()<2{continue;}let mut nums=items.iter().flat_map(|e|e.contributors.iter().filter_map(|c|c.module_number)).collect::<Vec<_>>();nums.sort();nums.dedup();let concepts=unique(items.iter().flat_map(|e|e.concept_terms.clone()).collect(),12);let id=stable_hash_id("abstraction",&sig);let now=now_ms();if let Some(a)=dev.abstractions.iter_mut().find(|a|a.id==id){a.instances=items.len() as u64;a.module_numbers=nums;a.concept_terms=concepts;a.confidence=(0.55+((items.len()-3) as f32*0.04).min(0.2)+(prompts.len() as f32*0.02).min(0.1)).min(0.93);a.updated_at=now;updated+=1;}else{dev.abstractions.push(ProvisionalAbstraction{id,signature:sig.clone(),kind:"experience_pattern".into(),status:"PROVISIONAL".into(),general_term:format!("experienced {} pattern",sig.split('|').next().unwrap_or("general")),description:"Repeated successful cognitive episodes used a similar learned route.".into(),module_numbers:nums,concept_terms:concepts,instances:items.len() as u64,confidence:0.55,evidence:items.iter().take(12).map(|e|json!({"type":"development_episode","id":e.id.clone(),"promptHash":e.prompt_hash.clone()})).collect(),created_at:now,updated_at:now});created+=1;}}
 if dev.abstractions.len()>MAX_ABSTRACTIONS{dev.abstractions=dev.abstractions.split_off(dev.abstractions.len()-MAX_ABSTRACTIONS);}let report=DevelopmentConsolidationReport{ran:true,reason:reason.into(),episodes_considered:dev.episodes.len(),routes_merged:merged,routes_pruned:pruned,abstractions_created:created,abstractions_updated:updated,completed_at:now_ms()};dev.since_consolidation=0;dev.last_consolidation=Some(report.clone());report }

pub fn reflect_completed_turn(dev:&mut RustDevelopmentState,learning:&mut RustLearningState,state:&CanonicalCognitiveState,report:&CognitiveRuntimeReport,turn_id:&str,raw_text:&str,supported:bool)->DevelopmentConsolidationReport {if !dev.initialized{*dev=RustDevelopmentState::new_initialized();}let s=signature(state);let route_id=if supported{reflect_success(learning,state,report)}else{reflect_failure(learning,report);None};let contributors=report.changed.iter().filter_map(|r|r.module_number.map(|n|ModuleContribution{module_id:r.module_id.clone(),module_number:Some(n),strength:(0.35+(r.delta_score.min(8) as f32)*0.07).min(1.0),state_change_count:r.delta_score})).collect::<Vec<_>>();let prompt_hash=stable_hash_id("prompt",raw_text);let id=stable_hash_id("episode",&format!("{turn_id}|{prompt_hash}"));dev.episodes.push(DevelopmentEpisode{id,turn_id:turn_id.into(),prompt_hash,prompt_preview:raw_text.trim().chars().take(180).collect(),supported,concept_terms:s.concepts.clone(),goal_terms:s.goals.clone(),relation_terms:s.relations.clone(),contributors,experience_route_ids:route_id.into_iter().collect(),cycles:report.cycles,created_at:now_ms()});if dev.episodes.len()>MAX_EPISODES{dev.episodes.remove(0);}for name in &state.active_categories{dev.register_category(name,"experienced","ACTIVE",vec![],vec![json!({"turnId":turn_id})]);}for row in &report.changed{for category in &row.categories{dev.register_category(category,"learned_module","ACTIVE",vec![],vec![json!({"moduleId":row.module_id.0.clone(),"turnId":turn_id})]);}}dev.since_consolidation=dev.since_consolidation.saturating_add(1);dev.revision=dev.revision.saturating_add(1);dev.updated_at=now_ms();learning.revision=learning.revision.saturating_add(1);learning.updated_at=now_ms();if dev.since_consolidation>=CONSOLIDATE_EVERY{consolidate(dev,learning,"periodic_rust_developmental_consolidation")}else{DevelopmentConsolidationReport{ran:false,reason:"not_due".into(),episodes_considered:dev.episodes.len(),..Default::default()}}}
