use field::Field;

#[derive(Debug, Clone, Copy)]
pub struct PublicInputs<F: Field>{
    pub trace_len: usize,
    pub start: F,
    pub step: F,   
}

impl<F: Field> PublicInputs<F>{
    pub fn end(&self) -> F {
        self.start + self.step * F::from_u64((self.trace_len - 1) as u64)  
    }    
}    

pub fn generate_trace<F: Field>(public: &PublicInputs<F>) -> Vec<F>{
    let mut trace = Vec::with_capacity(public.trace_len);
    let mut value = public.start;
    for _ in 0..public.trace_len{
        trace.push(value);
        value += public.step;
    }        
    trace
}      

pub fn evaluate_composition<F: Field>(
    trace_lde_evals: &[F],
    trace_lde_domain: &[F],
    blowup_factor: usize,
    public: &PublicInputs<F>,
    alphas: (F, F, F),  
) -> Vec<F> {
    let big_n = trace_lde_evals.len();
    assert_eq!(trace_lde_domain.len(), big_n);
    assert_eq!(big_n % blowup_factor, 0);
    
    let n = public.trace_len;
    let log_n = (n as u64).trailing_zeros();
    let trace_generator = F::root_of_unity(log_n);
    let last_trace_point = trace_generator.pow((n-1) as u64);
    let end_value =  public.end();
    
    let mut composition = Vec::with_capacity(big_n);
    for k in 0..big_n{
        let x = trace_lde_domain[k];
        let a_here = trace_lde_evals[k];
        let a_next = trace_lde_evals[(k + blowup_factor) % big_n];
        composition.push(evaluate_composition_at_point(
            x,
            a_here,
            a_next,
            n,
            last_trace_point,
            end_value,
            public,
            alphas,
        ));
    }
    composition
} 

#[allow(clippy::too_many_arguments)]
fn evaluate_composition_at_point<F: Field>(
    x: F,
    a_here: F,
    a_next: F,
    n: usize,
    last_trace_point: F,
    end_value: F,
    public: &PublicInputs<F>,
    alphas: (F, F, F), 
) -> F{
    let (alpha_transition, alpha_start, alpha_end) = alphas;
    let vanishing_h = x.pow(n as u64) - F::one();
    let transition_num = a_next - a_here - public.step;
    let transition_zerofier = vanishing_h
        * (x - last_trace_point)
            .inverse()
            .expect("x is an LDE-domain point in a coset disjoint from H, so x != ω^(n-1)");
    let transition_quotient = transition_num
        * transition_zerofier
            .inverse()
            .expect("vanishing_h/(x-last) is nonzero off of H, and the LDE coset avoids H entirely");
            
    let start_num = a_here - public.start;
    let start_quotient = start_num * (x - F::one()).inverse().expect("x != 1 on the LDE coset");        

    let end_num = a_here - end_value;
    let end_quotient = end_num * (x - last_trace_point).inverse().expect("x != ω^(n-1) on the LDE coset");

    alpha_transition * transition_quotient + alpha_start * start_quotient + alpha_end * end_quotient
}

pub fn evaluate_composition_at_query_point<F: Field>(
    x: F,
    a_here: F,
    a_next: F,
    public: &PublicInputs<F>,
    alphas: (F, F, F),
) -> F {
    let n = public.trace_len;
    let log_n = (n as u64).trailing_zeros();
    let last_trace_point = F::root_of_unity(log_n).pow((n-1) as u64);
    evaluate_composition_at_point(x, a_here, a_next, n, last_trace_point, public.end(), public, alphas)   
}   

#[cfg(test)]
mod tests {
    use super::*;
    use field::ToyField;
    use poly::interpolate::fft_interpolate;
    use poly::lde::{coset_domain, low_degree_extend};

    fn tf(x: u64) -> ToyField {
        ToyField::from_u64(x)
    }

    fn honest_setup() -> (PublicInputs<ToyField>, Vec<ToyField>) {
        let public = PublicInputs { trace_len: 8, start: tf(3), step: tf(2) };
        let trace = generate_trace(&public);
        (public, trace)
    }

    #[test]
    fn generated_trace_matches_arithmetic_progression() {
        let (public, trace) = honest_setup();
        for (i, &v) in trace.iter().enumerate() {
            assert_eq!(v, public.start + public.step * tf(i as u64));
        }
        assert_eq!(*trace.last().unwrap(), public.end());
    }

    #[test]
    fn honest_trace_gives_low_degree_composition() {
        let (public, trace) = honest_setup();
        let n = trace.len();
        let blowup = 4;
        let offset = tf(5);

        let trace_coeffs = fft_interpolate(&trace).coeffs().to_vec();
        let lde_evals = low_degree_extend(&trace_coeffs, blowup, offset);
        let lde_domain = coset_domain(n * blowup, offset);

        let alphas = (tf(11), tf(13), tf(17));
        let composition = evaluate_composition(&lde_evals, &lde_domain, blowup, &public, alphas);

        let interpolated = poly::interpolate::lagrange_interpolate(
            &lde_domain.iter().copied().zip(composition.iter().copied()).collect::<Vec<_>>(),
        );
        assert!(
            interpolated.degree() < n * 2,
            "composition degree {} unexpectedly high for an honest trace",
            interpolated.degree()
        );
    }

    #[test]
    fn tampered_trace_gives_high_degree_composition() {
        let (public, mut trace) = honest_setup();
        trace[4] += tf(1); 
        let n = trace.len();
        let blowup = 4;
        let offset = tf(5);

        let trace_coeffs = fft_interpolate(&trace).coeffs().to_vec();
        let lde_evals = low_degree_extend(&trace_coeffs, blowup, offset);
        let lde_domain = coset_domain(n * blowup, offset);

        let alphas = (tf(11), tf(13), tf(17));
        let composition = evaluate_composition(&lde_evals, &lde_domain, blowup, &public, alphas);

        let interpolated = poly::interpolate::lagrange_interpolate(
            &lde_domain.iter().copied().zip(composition.iter().copied()).collect::<Vec<_>>(),
        );
        assert!(
            interpolated.degree() >= n * blowup - 2,
            "expected a tampered trace to blow up the composition's degree, got {}",
            interpolated.degree()
        );
    }
}