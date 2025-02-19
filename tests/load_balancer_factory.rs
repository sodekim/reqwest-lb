use reqwest_lb::{supplier::LoadBalancer, LoadBalancerPolicy, LoadBalancerRegistry};

#[test]
fn load_balancer_factory() {
    let mut factory = LoadBalancerRegistry::default();
    let load_balancer = LoadBalancer::new(Vec::<usize>::new(), LoadBalancerPolicy::default());
    factory.add("example", load_balancer);
    let load_balancer = factory.find("example");
    assert!(load_balancer.is_some());
    let load_balancer = factory.find("test");
    assert!(load_balancer.is_none());
}
