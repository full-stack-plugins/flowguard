//! Actual pinned SpecGuard APIs over a committed local fixture; authority is explicit fixture-only.
use specguard::{baseline::*, graph::*, integration::approval::*, model::*, obligations::*, source::*};
use std::{collections::BTreeSet,path::Path};
struct Fixture;
impl ApprovalValidationPort for Fixture {
 fn profile(&self)->Profile{Profile::Fixture}
 fn validate(&self,b:&ApprovedBaseline)->Result<Authentication,ApprovalError>{Ok(Authentication{issuer:"fixture-controller".into(),purpose:"specification-baseline".into(),repository:b.repository.clone(),scope:b.scope.clone(),baseline_digest:digest(b),policy_digest:b.policy_digest.clone(),issued_at:10,expires_at:100,revoked:false})}
}
fn main(){
 let args:Vec<_>=std::env::args().collect();let root=Path::new(&args[1]);let out=Path::new(&args[2]);
 let oid=std::process::Command::new("git").arg("-C").arg(root).args(["rev-parse","HEAD"]).output().unwrap();assert!(oid.status.success());let oid=String::from_utf8(oid.stdout).unwrap().trim().to_owned();
 let policy=SourcePolicy{api_version:Version::V1,roots:vec![SourceRoot{path:"specs".into(),format:"markdown-explicit/v1".into(),namespace:"demo".into(),authority:"primary".into()}],limits:Limits::default()};
 let inventory=discover(root,&policy).unwrap();
 let snapshot=freeze(root,&inventory,CandidateBinding{candidate_oid:oid.clone(),base_oid:oid.clone(),object_format:"sha1".into()}).unwrap();let status=std::process::Command::new("git").arg("-C").arg(root).args(["status","--porcelain"]).output().unwrap();assert!(status.status.success() && status.stdout.is_empty());
 let graph=build_graph(specguard::parser::parse(&snapshot));assert!(graph.complete());
 let scope:BTreeSet<_>=graph.parsed.requirements.iter().map(|r|r.key.clone()).collect();
 let b=ApprovedBaseline{api_version:Version::V1,repository:"fixture:flowguard-import".into(),scope,source_revision:oid,source_digest:graph.parsed.snapshot_digest.clone(),graph_digest:digest(&graph),policy_digest:format!("sha256:{}","a".repeat(64)),approval_ref:"fixture:approval-flowguard".into(),effective_from:10,expires_at:100,state:BaselineState::Approved,graph};
 let approved=authenticate(&b,&Fixture,Profile::Fixture,50).unwrap();
 let e=export_obligations(&b.graph,&approved,&b.scope,&b.source_digest).unwrap();assert_eq!(e.obligations.len(),3);assert!(e.complete);
 std::fs::create_dir_all(out).unwrap();
 std::fs::write(out.join("baseline.json"),serde_json::to_vec_pretty(&b).unwrap()).unwrap();
 std::fs::write(out.join("obligations.json"),serde_json::to_vec_pretty(&e).unwrap()).unwrap();
}
