use reqwest_lb::{LoadBalancerFactory, LoadBalancerPolicy, SimpleLoadBalancer};

#[test]
fn load_balancer_factory() {
    let mut factory = LoadBalancerFactory::default();
    let load_balancer = SimpleLoadBalancer::new(Vec::<usize>::new(), LoadBalancerPolicy::default());
    factory.add("example", load_balancer);
    let load_balancer = factory.get("example");
    assert!(load_balancer.is_some());
    let load_balancer = factory.get("test");
    assert!(load_balancer.is_none());
}
